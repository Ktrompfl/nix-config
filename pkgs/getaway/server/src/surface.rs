use crate::commit::{self, Point, Queued};
use crate::{
    ClientId, Key, State,
    buffer::Buffer,
    client::Data,
    constraints,
    privileged::{
        layer_shell::{self, Layer, LayerChanges},
        session_lock,
    },
    seat, shell,
    xdg::{self, Popup, Toplevel},
};
use std::{collections::VecDeque, mem, ops::Deref, rc::Rc};
use wire::*;
use wm::{Rect, Region};

pub mod configure;

const HISTORY: usize = 8;
const MAX_QUEUED: usize = 32;
const MAX_DAMAGE: i64 = 1 << 20;

macro_rules! double_buffered {
    ($changes:ident => $state:ident { $($field:ident: $ty:ty = $default:expr),* $(,)? }) => {
        #[derive(Clone)]
        pub struct $state { $(pub $field: $ty),* }

        impl Default for $state {
            fn default() -> Self { Self { $($field: $default),* } }
        }

        #[derive(Clone, Default)]
        pub struct $changes { $(pub $field: Option<$ty>),* }

        impl $changes {
            pub fn merge(&mut self, older: Self) { $(self.$field = self.$field.take().or(older.$field);)* }

            pub fn apply(self, state: &mut $state) { $(if let Some(v) = self.$field { state.$field = v; })* }

            pub fn over(&self, mut base: $state) -> $state { self.clone().apply(&mut base); base }
        }
    };
}
pub(crate) use double_buffered;

double_buffered! {
    Changes => Latched {
        scale: i32 = 1,
        transform: u32 = 0,
        source: Option<[f64; 4]> = None,
        destination: Option<(i32, i32)> = None,
        alpha: f64 = 1.0,
        content_type: u32 = 0,
        tearing: bool = false,
        geometry: Rect = Rect::default(),
        input: Option<Region> = None,
        configure: Option<u32> = None,
        pointer_hint: Option<(u32, (f64, f64))> = None,
        pointer_region: Option<(u32, Option<Region>)> = None,
    }
}

#[derive(Default)]
pub struct Pending {
    pub state: Changes,
    pub layer: LayerChanges,
    buffer: Option<Option<Rc<Buffer>>>,
    releases: Vec<u32>,
    pub acquire: Option<Point>,
    pub release: Option<Point>,
    callbacks: Vec<u32>,
    pub feedbacks: Vec<u32>,
    offset: (i32, i32),
    damage: Rect,
    surface_damage: Rect,
    order: Option<Vec<Option<u32>>>,
    positions: Vec<(u32, (i32, i32))>,
    added: Vec<u32>,
    pub subsurfaces: Vec<(u32, Pending)>,
    pub fifo_barrier: bool,
    pub fifo_wait: bool,
    pub timestamp: Option<u64>,
}

impl Pending {
    pub fn attached(&self) -> Option<&Rc<Buffer>> {
        self.buffer.as_ref()?.as_ref()
    }
}

#[derive(Default)]
pub struct Release {
    point: Option<Point>,
    callbacks: Vec<u32>,
}

pub type Retired = (ClientId, Rc<Buffer>, Release);

#[derive(Default)]
pub struct Surface {
    pub pending: Pending,
    pub held: Option<Pending>,
    cached: Option<Pending>,
    pub state: Latched,
    pub buffer: Option<Rc<Buffer>>,
    releases: Vec<u32>,
    pub callbacks: Vec<u32>,
    pub feedbacks: Vec<u32>,
    pub children: Vec<u32>,
    pub below: usize,
    pub role: Role,
    pub version: u32,
    pub inhibitors: u32,
    pub fractional: Option<(u32, u32)>,
    pub syncobj: Option<u32>,
    pub constraint: Option<constraints::Constraint>,
    pub queue: VecDeque<Queued>,
    pub barrier: Vec<(backend::OutputId, u64)>,
    release_point: Option<Point>,
    pub commits: u64,
    history: VecDeque<Rect>,
    pub outputs: Vec<backend::OutputId>,
    pub entered: Vec<u32>,
    pub buffer_scale_sent: i32,
    pub preferred_scale: f64,
}

impl Deref for Surface {
    type Target = Latched;

    fn deref(&self) -> &Latched {
        &self.state
    }
}

impl Surface {
    pub fn new(version: u32) -> Self {
        Self { version, preferred_scale: 1.0, ..Default::default() }
    }

    pub fn size(&self) -> (i32, i32) {
        size(&self.state, self.buffer.as_deref()).unwrap_or((0, 0))
    }

    pub fn source(&self) -> [f64; 4] {
        let (w, h) = self.buffer.as_ref().map_or((0, 0), |b| b.size());
        let (w, h) = (f64::from(w), f64::from(h));
        let Some(source) = self.state.source else { return [0.0, 0.0, w, h] };
        let [x, y, sw, sh] = source.map(|v| v * f64::from(self.state.scale));
        match self.state.transform {
            1 => [y, h - x - sw, sh, sw],
            2 => [w - x - sw, h - y - sh, sw, sh],
            3 => [w - y - sh, x, sh, sw],
            4 => [w - x - sw, y, sw, sh],
            5 => [y, x, sh, sw],
            6 => [x, h - y - sh, sw, sh],
            7 => [w - y - sh, h - x - sw, sh, sw],
            _ => [x, y, sw, sh],
        }
    }

    fn stack(&self) -> Vec<Option<u32>> {
        let (below, above) = self.children.split_at(self.below.min(self.children.len()));
        below.iter().copied().map(Some).chain([None]).chain(above.iter().copied().map(Some)).collect()
    }

    fn all_children(&self) -> Vec<u32> {
        self.children.iter().chain(&self.pending.added).copied().collect()
    }
}

fn size(state: &Latched, buffer: Option<&Buffer>) -> Option<(i32, i32)> {
    match (state.destination, state.source) {
        (Some(size), _) => Some(size),
        (None, Some([.., w, h])) => Some((w as i32, h as i32)),
        (None, None) => buffer.map(|b| {
            let (w, h) = transformed(b.size(), state.transform);
            (w / state.scale, h / state.scale)
        }),
    }
}

fn transformed((w, h): (i32, i32), transform: u32) -> (i32, i32) {
    if transform % 2 == 1 { (h, w) } else { (w, h) }
}

#[derive(Default)]
pub enum Role {
    #[default]
    None,
    Subsurface {
        id: u32,
        parent: u32,
        x: i32,
        y: i32,
        sync: bool,
    },
    Toplevel(Toplevel),
    Popup(Popup),
    Layer(Layer),
    LockSurface(session_lock::LockSurface),
    Cursor,
    DndIcon,
}

impl Role {
    pub fn take_toplevel(&mut self) -> Option<Toplevel> {
        match mem::take(self) {
            Role::Toplevel(t) => Some(t),
            other => {
                *self = other;
                None
            }
        }
    }
}

fn clamped(x: i64, y: i64, w: i64, h: i64) -> Rect {
    let c = |v: i64| v.clamp(-MAX_DAMAGE, MAX_DAMAGE) as i32;
    Rect::new(c(x), c(y), c(w.max(0)), c(h.max(0)))
}

impl wl_surface::Handler for State {
    fn attach(&mut self, cx: Cx, buffer: Option<u32>, x: i32, y: i32) -> Res {
        if cx.obj.version >= 5 && (x, y) != (0, 0) {
            let message = "attach offset requires offset request";
            return Err(cx.error(wl_surface::error::INVALID_OFFSET, message));
        }
        let unknown = |id| Error::new(DISPLAY.id, wl_display::error::INVALID_OBJECT, format!("{id} is not a buffer"));
        let buffer = match buffer.map(|id| (id, self.data_mut((cx.client, id)))) {
            None => None,
            Some((_, Some(Data::Buffer(b)))) => Some(b.clone()),
            Some((id, _)) => return Err(unknown(id)),
        };
        let pending = pending(self, cx);
        pending.buffer = Some(buffer);
        pending.offset = (pending.offset.0 + x, pending.offset.1 + y);
        Ok(())
    }

    fn damage(&mut self, cx: Cx, x: i32, y: i32, width: i32, height: i32) {
        let pending = pending(self, cx);
        let damage = clamped(x.into(), y.into(), width.into(), height.into());
        pending.surface_damage = pending.surface_damage.union(damage);
    }

    fn frame(&mut self, cx: Cx, callback: Obj) {
        pending(self, cx).callbacks.push(callback.id);
    }

    fn set_opaque_region(&mut self, _: Cx, _: Option<u32>) {}

    fn set_input_region(&mut self, cx: Cx, region: Option<u32>) {
        let region = crate::base::region(self, cx.client, region);
        pending(self, cx).state.input = Some(region);
    }

    fn commit(&mut self, cx: Cx) -> Res {
        let key = cx.key();
        let s = self.surfaces.get(&key).expect("surface object has state");
        if !s.pending.releases.is_empty() && s.pending.attached().is_none() {
            return Err(cx.error(wl_surface::error::NO_BUFFER, "release requested without a buffer"));
        }
        if s.queue.len() >= MAX_QUEUED {
            return Err(Error::new(DISPLAY.id, wl_display::error::NO_MEMORY, "too many queued commits"));
        }
        commit::check_points(s, &s.pending)?;
        let next = upcoming(s).fold(s.state.clone(), |base, p| p.state.over(base));
        let buffer = upcoming(s).rev().find_map(|p| p.buffer.as_ref()).unwrap_or(&s.buffer).as_deref();
        let viewport = || extension(self, key, Interface::WpViewport).map_or(0, |v| v.id);
        check_buffer(cx.obj, viewport, s, &next, buffer)?;
        let size = buffer.and(size(&next, buffer));
        layer_shell::check_commit(self, key, size)?;
        let configure = xdg::check_commit(self, key, size.is_some())?;
        session_lock::check_commit(self, key, size)?;
        pending(self, cx).state.configure = Some(configure);
        commit(self, key);
        Ok(())
    }

    fn set_buffer_transform(&mut self, cx: Cx, transform: i32) -> Res {
        cx.error_if(!(0..8).contains(&transform), wl_surface::error::INVALID_TRANSFORM, "invalid buffer transform")?;
        pending(self, cx).state.transform = Some(transform as u32);
        Ok(())
    }

    fn set_buffer_scale(&mut self, cx: Cx, scale: i32) -> Res {
        cx.error_if(scale < 1, wl_surface::error::INVALID_SCALE, "buffer scale must be positive")?;
        pending(self, cx).state.scale = Some(scale);
        Ok(())
    }

    fn damage_buffer(&mut self, cx: Cx, x: i32, y: i32, width: i32, height: i32) {
        let pending = pending(self, cx);
        pending.damage = pending.damage.union(clamped(x.into(), y.into(), width.into(), height.into()));
    }

    fn offset(&mut self, cx: Cx, x: i32, y: i32) {
        let pending = pending(self, cx);
        pending.offset = (pending.offset.0 + x, pending.offset.1 + y);
    }

    fn get_release(&mut self, cx: Cx, callback: Obj) {
        pending(self, cx).releases.push(callback.id);
    }
}

fn check_buffer(obj: Obj, viewport: impl Fn() -> u32, s: &Surface, next: &Latched, buffer: Option<&Buffer>) -> Res {
    use wp_viewport::error::*;
    let Some(((w, h), scale)) = buffer.map(|b| (b.size(), next.scale)) else { return Ok(()) };
    if !matches!(s.role, Role::Cursor) && (w % scale != 0 || h % scale != 0) {
        return Err(Error::new(obj.id, wl_surface::error::INVALID_SIZE, "buffer size is not a multiple of its scale"));
    }
    let Some([x, y, sw, sh]) = next.source else { return Ok(()) };
    let ((bw, bh), scale) = (transformed((w, h), next.transform), f64::from(scale));
    if x + sw > f64::from(bw) / scale || y + sh > f64::from(bh) / scale {
        return Err(Error::new(viewport(), OUT_OF_BUFFER, "source rectangle extends outside of the buffer"));
    }
    if next.destination.is_none() && (sw.fract() != 0.0 || sh.fract() != 0.0) {
        return Err(Error::new(viewport(), BAD_SIZE, "source size is not integer and no destination is set"));
    }
    Ok(())
}

pub fn upcoming(s: &Surface) -> impl DoubleEndedIterator<Item = &Pending> {
    s.held.iter().chain(s.queue.iter().map(|q| &q.pending)).chain(&s.cached).chain([&s.pending])
}

fn pending(st: &mut State, cx: Cx) -> &mut Pending {
    &mut st.surfaces.get_mut(&cx.key()).expect("surface object has state").pending
}

fn commit(st: &mut State, key: Key) {
    let s = st.surfaces.get_mut(&key).expect("surface object has state");
    let children = s.all_children();
    let mut pending = mem::take(&mut s.pending);
    pending.subsurfaces = children
        .into_iter()
        .filter_map(|child| Some((child, st.surfaces.get_mut(&(key.0, child))?.cached.take()?)))
        .collect();
    if !synced(st, key) {
        return commit::submit(st, key, pending);
    }
    let mut cached = st.surfaces.get_mut(&key).and_then(|s| s.cached.take()).unwrap_or_default();
    merge(st, key.0, &mut cached, pending);
    st.surfaces.get_mut(&key).expect("surface object has state").cached = Some(cached);
}

fn synced(st: &State, key: Key) -> bool {
    match st.surfaces.get(&key).map(|s| &s.role) {
        Some(&Role::Subsurface { parent, sync, .. }) if st.surfaces.contains_key(&(key.0, parent)) => {
            sync || synced(st, (key.0, parent))
        }
        _ => false,
    }
}

fn desynced(st: &mut State, key: Key) {
    if synced(st, key) {
        return;
    }
    let Some(s) = st.surfaces.get_mut(&key) else { return };
    let (cached, children) = (s.cached.take(), s.all_children());
    if let Some(cached) = cached {
        commit::submit(st, key, cached);
    }
    children.into_iter().for_each(|child| desynced(st, (key.0, child)));
}

fn child_of(st: &State, key: Key, child: u32) -> bool {
    matches!(st.surfaces.get(&(key.0, child)).map(|s| &s.role), Some(&Role::Subsurface { parent, .. }) if parent == key.1)
}

pub fn latch(st: &mut State, key: Key, pending: Pending) {
    if let Some(pending) = shell::hold(st, key, pending) {
        apply(st, key, pending);
    }
}

pub fn apply(st: &mut State, key: Key, pending: Pending) {
    let added: Vec<u32> = pending.added.iter().copied().filter(|&child| child_of(st, key, child)).collect();
    position_children(st, key, pending.positions);
    let Some(surface) = st.surfaces.get_mut(&key) else { return };
    for child in added {
        if !surface.children.contains(&child) {
            surface.children.push(child);
        }
    }
    surface.commits += 1;
    surface.callbacks.extend(pending.callbacks);
    let replaced = mem::replace(&mut surface.feedbacks, pending.feedbacks);
    pending.state.apply(&mut surface.state);
    if let Some(order) = pending.order {
        restack(surface, order);
    }
    let s = &surface.state;
    let viewported = s.source.is_some() || s.destination.is_some() || s.transform != 0;
    let scaled = match viewported {
        true if !pending.surface_damage.is_empty() => clamped(0, 0, MAX_DAMAGE, MAX_DAMAGE),
        _ => {
            let (r, s) = (pending.surface_damage, i64::from(s.scale));
            clamped(i64::from(r.x) * s, i64::from(r.y) * s, i64::from(r.w) * s, i64::from(r.h) * s)
        }
    };
    let damage = pending.damage.union(scaled);
    discard(st, key.0, replaced);
    if pending.offset != (0, 0) {
        crate::cursor::offset(st, key, pending.offset);
        crate::dnd::offset(st, key, pending.offset);
    }
    let Some(surface) = st.surfaces.get_mut(&key) else { return };
    let old = surface.buffer.clone();
    if let Some(new) = &pending.buffer {
        surface.buffer = new.clone();
    }
    track_damage(surface, key, old.as_ref(), pending.buffer.is_some(), damage);
    if let Some(new) = pending.buffer {
        let release = Release {
            point: mem::replace(&mut surface.release_point, pending.release),
            callbacks: mem::replace(&mut surface.releases, pending.releases),
        };
        let reattached = old.as_ref().zip(new.as_ref()).is_some_and(|(a, b)| Rc::ptr_eq(a, b));
        if new.is_none()
            && let (Some(old), Some(w)) = (&old, xdg::window_of(st, key))
        {
            shell::detached(st, w, old.clone());
        }
        match old.filter(|_| !reattached) {
            Some(old) => retire(st, key.0, old, release),
            None => fire(st, key.0, release),
        }
    }
    for (child, state) in pending.subsurfaces {
        match child_of(st, key, child) {
            true => apply(st, (key.0, child), state),
            false => abandon(st, key.0, state),
        }
    }
    st.render_dirty = true;
    constraints::update(st);
    xdg::committed(st, key);
    layer_shell::committed(st, key, pending.layer);
}

fn track_damage(surface: &mut Surface, key: Key, old: Option<&Rc<Buffer>>, attached: bool, damage: Rect) {
    let Some(buffer) = surface.buffer.clone() else {
        surface.history.push_front(clamped(0, 0, MAX_DAMAGE, MAX_DAMAGE));
        return surface.history.truncate(HISTORY);
    };
    let (w, h) = buffer.size();
    let full = Rect::new(0, 0, w, h);
    let resized = old.is_none_or(|o| o.size() != (w, h));
    let damage = if resized { full } else { damage.intersect(full) };
    let reattached = old.is_some_and(|o| Rc::ptr_eq(o, &buffer));
    let since = |n: u64| {
        let age =
            usize::try_from(surface.commits - n).ok().filter(|&a| (1..=surface.history.len() + 1).contains(&a))?;
        Some(surface.history.iter().take(age - 1).fold(damage, |r, d| r.union(*d)))
    };
    let region = match buffer.last() {
        _ if !attached || reattached => damage,
        _ if damage.is_empty() => full,
        Some((k, n)) if k == key => since(n).unwrap_or(full),
        _ => full,
    };
    buffer.damage(region.intersect(full));
    buffer.set_last(key, surface.commits);
    surface.history.push_front(damage);
    surface.history.truncate(HISTORY);
}

fn restack(surface: &mut Surface, order: Vec<Option<u32>>) {
    let mut children = Vec::with_capacity(surface.children.len());
    let mut below = 0;
    for entry in order {
        match entry {
            None => below = children.len(),
            Some(id) if surface.children.contains(&id) && !children.contains(&id) => children.push(id),
            Some(_) => {}
        }
    }
    let added: Vec<u32> = surface.children.iter().copied().filter(|c| !children.contains(c)).collect();
    children.extend(added);
    (surface.children, surface.below) = (children, below);
}

fn position_children(st: &mut State, key: Key, positions: Vec<(u32, (i32, i32))>) {
    for (child, position) in positions {
        if let Some(Role::Subsurface { parent, x, y, .. }) = st.surfaces.get_mut(&(key.0, child)).map(|s| &mut s.role)
            && *parent == key.1
        {
            (*x, *y) = position;
        }
    }
}

pub fn merge(st: &mut State, c: ClientId, into: &mut Pending, mut pending: Pending) {
    pending.state.merge(mem::take(&mut into.state));
    pending.layer.merge(mem::take(&mut into.layer));
    pending.callbacks.splice(0..0, into.callbacks.drain(..));
    pending.offset = (pending.offset.0 + into.offset.0, pending.offset.1 + into.offset.1);
    pending.damage = pending.damage.union(into.damage);
    pending.surface_damage = pending.surface_damage.union(into.surface_damage);
    pending.order = pending.order.take().or(into.order.take());
    pending.positions.splice(0..0, into.positions.drain(..));
    pending.added.splice(0..0, into.added.drain(..));
    pending.subsurfaces.splice(0..0, into.subsurfaces.drain(..));
    let older = mem::replace(into, pending);
    discard(st, c, older.feedbacks);
    let Some(new) = &into.buffer else {
        (into.buffer, into.releases, into.release, into.acquire) =
            (older.buffer, older.releases, older.release, older.acquire);
        return;
    };
    commit::release(older.release);
    let reattached = |old: &Rc<Buffer>| new.as_ref().is_some_and(|n| Rc::ptr_eq(n, old));
    if let Some(Some(old)) = older.buffer.filter(|old| !old.as_ref().is_some_and(reattached))
        && !in_use(st, &old)
    {
        release(st, c, &old);
    }
    callbacks_done(st, c, older.releases);
}

fn in_use(st: &State, buffer: &Rc<Buffer>) -> bool {
    let shown = st.surfaces.values().filter_map(|s| s.buffer.as_ref());
    shown.chain(st.retired.iter().map(|r| &r.1)).any(|b| Rc::ptr_eq(b, buffer))
}

fn callbacks_done(st: &mut State, c: ClientId, callbacks: Vec<u32>) {
    for callback in callbacks {
        crate::base::callback_done(st, c, callback, 0);
    }
}

pub fn discard(st: &mut State, c: ClientId, feedbacks: Vec<u32>) {
    let Some(client) = st.clients.get_mut(&c) else { return };
    for id in feedbacks {
        if let Some(feedback) = client.get(id).map(|o| o.obj) {
            wp_presentation_feedback::discarded(&mut client.out, feedback);
            client.delete(id);
        }
    }
}

fn fire(st: &mut State, c: ClientId, release: Release) {
    commit::release(release.point);
    callbacks_done(st, c, release.callbacks);
}

fn retire(st: &mut State, c: ClientId, buffer: Rc<Buffer>, rel: Release) {
    match buffer.busy(&st.renderer) || st.backend.scanned_out().contains(&buffer.serial) {
        true => st.retired.push((c, buffer, rel)),
        false => {
            release(st, c, &buffer);
            fire(st, c, rel);
        }
    }
}

pub fn release_retired(st: &mut State) {
    if st.retired.is_empty() {
        return;
    }
    let (scanned_out, renderer) = (st.backend.scanned_out(), &st.renderer);
    let done: Vec<_> =
        st.retired.extract_if(.., |r| !r.1.busy(renderer) && !scanned_out.contains(&r.1.serial)).collect();
    for (c, buffer, rel) in done {
        release(st, c, &buffer);
        fire(st, c, rel);
    }
}

pub fn release(st: &mut State, c: ClientId, buffer: &Rc<Buffer>) {
    if matches!(st.data_mut((c, buffer.id)), Some(Data::Buffer(b)) if Rc::ptr_eq(b, buffer)) {
        st.send(c, |out| wl_buffer::release(out, Obj { id: buffer.id, version: 1 }));
    }
}

pub fn destroy(st: &mut State, key: Key) {
    xdg::unmap(st, key);
    layer_shell::unmap(st, key);
    let Some(surface) = st.surfaces.remove(&key) else { return };
    if let Some(client) = st.clients.get_mut(&key.0) {
        client.objects_mut().filter(|o| o.data == key.1).for_each(|o| o.data = 0);
    }
    if let Role::Subsurface { parent, .. } = surface.role {
        detach(st, key, parent);
    }
    let children = surface.all_children();
    for &child in &children {
        if let Some(Role::Subsurface { parent, .. }) = st.surfaces.get_mut(&(key.0, child)).map(|s| &mut s.role)
            && *parent == key.1
        {
            *parent = 0;
        }
    }
    children.into_iter().for_each(|child| desynced(st, (key.0, child)));
    let rel = Release { point: surface.release_point, callbacks: surface.releases };
    match surface.buffer {
        Some(buffer) => retire(st, key.0, buffer, rel),
        None => fire(st, key.0, rel),
    }
    commit::forget(st, key);
    let committed = surface.held.into_iter().chain(surface.cached).chain(surface.queue.into_iter().map(|q| q.pending));
    committed.for_each(|pending| abandon(st, key.0, pending));
    let pending = surface.pending;
    callbacks_done(st, key.0, pending.releases);
    commit::release(pending.release);
    discard(st, key.0, [surface.feedbacks, pending.feedbacks].concat());
    forget_callbacks(st, key.0, [surface.callbacks, pending.callbacks].concat());
    seat::surface_gone(st, key);
    crate::dnd::surface_gone(st, key);
    constraints::remove(st, key);
    st.render_dirty = true;
}

fn abandon(st: &mut State, c: ClientId, pending: Pending) {
    for pending in flatten(pending) {
        if let Some(buffer) = pending.attached().filter(|b| !in_use(st, b)) {
            release(st, c, buffer);
        }
        callbacks_done(st, c, pending.releases);
        commit::release(pending.release);
        discard(st, c, pending.feedbacks);
        forget_callbacks(st, c, pending.callbacks);
    }
}

fn flatten(mut pending: Pending) -> Vec<Pending> {
    let nested = mem::take(&mut pending.subsurfaces);
    [pending].into_iter().chain(nested.into_iter().flat_map(|(_, p)| flatten(p))).collect()
}

fn forget_callbacks(st: &mut State, c: ClientId, callbacks: Vec<u32>) {
    let Some(client) = st.clients.get_mut(&c) else { return };
    for id in callbacks {
        client.delete(id);
    }
}

fn detach(st: &mut State, key: Key, parent: u32) {
    let Some(parent) = st.surfaces.get_mut(&(key.0, parent)) else { return };
    if let Some(at) = parent.children.iter().position(|&c| c == key.1) {
        parent.children.remove(at);
        parent.below -= usize::from(at < parent.below);
    }
}

pub fn extension(st: &State, key: Key, interface: Interface) -> Option<Obj> {
    let mut objects = st.clients.get(&key.0)?.objects();
    objects.find(|o| o.interface == interface && o.data == key.1).map(|o| o.obj)
}

pub fn add_extension(st: &mut State, cx: Cx, id: Obj, surface: u32, code: u32) -> Result<&mut Surface, Error> {
    let interface = st.client(cx.client).get(id.id).expect("new object exists").interface;
    if !st.surfaces.contains_key(&(cx.client, surface)) {
        let message = format!("{surface} is not a surface");
        return Err(Error::new(DISPLAY.id, wl_display::error::INVALID_OBJECT, message));
    }
    if extension(st, (cx.client, surface), interface).is_some() {
        return Err(cx.error(code, format!("surface already has a {}", interface.name())));
    }
    st.client(cx.client).set_data(id.id, surface);
    Ok(st.surfaces.get_mut(&(cx.client, surface)).expect("checked above"))
}

pub fn extension_gone(st: &mut State, key: Key, interface: Interface) {
    let Some(s) = st.surfaces.get_mut(&key) else { return };
    match interface {
        Interface::ZwpIdleInhibitorV1 => s.inhibitors -= 1,
        Interface::WpFractionalScaleV1 => s.fractional = None,
        _ => s.syncobj = None,
    }
}

pub fn extended(st: &mut State, cx: Cx, gone: Option<u32>) -> Result<Option<&mut Surface>, Error> {
    match (st.surfaces.get_mut(&cx.surface()), gone) {
        (None, Some(code)) => Err(cx.error(code, "surface is gone")),
        (s, _) => Ok(s),
    }
}

impl wp_alpha_modifier_v1::Handler for State {
    fn get_surface(&mut self, cx: Cx, id: Obj, surface: u32) -> Res {
        add_extension(self, cx, id, surface, wp_alpha_modifier_v1::error::ALREADY_CONSTRUCTED).map(drop)
    }
}

impl wp_alpha_modifier_surface_v1::Handler for State {
    fn destroy(&mut self, cx: Cx) -> Res {
        if let Some(s) = extended(self, cx, None)? {
            s.pending.state.alpha = Some(1.0);
        }
        Ok(())
    }

    fn set_multiplier(&mut self, cx: Cx, factor: u32) -> Res {
        if let Some(s) = extended(self, cx, Some(wp_alpha_modifier_surface_v1::error::NO_SURFACE))? {
            s.pending.state.alpha = Some(f64::from(factor) / f64::from(u32::MAX));
        }
        Ok(())
    }
}

impl wp_content_type_manager_v1::Handler for State {
    fn get_surface_content_type(&mut self, cx: Cx, id: Obj, surface: u32) -> Res {
        add_extension(self, cx, id, surface, wp_content_type_manager_v1::error::ALREADY_CONSTRUCTED).map(drop)
    }
}

impl wp_content_type_v1::Handler for State {
    fn destroy(&mut self, cx: Cx) -> Res {
        if let Some(s) = extended(self, cx, None)? {
            s.pending.state.content_type = Some(wp_content_type_v1::r#type::NONE);
        }
        Ok(())
    }

    fn set_content_type(&mut self, cx: Cx, content_type: u32) -> Res {
        if let Some(s) = extended(self, cx, None)? {
            s.pending.state.content_type = Some(content_type);
        }
        Ok(())
    }
}

pub fn assign_role(st: &mut State, key: Key, role: Role) -> Result<(), &'static str> {
    let surface = st.surfaces.get_mut(&key).ok_or("unknown surface")?;
    if !matches!(surface.role, Role::None) {
        return Err("surface already has a role");
    }
    surface.role = role;
    Ok(())
}

fn descends(st: &State, c: ClientId, from: u32, ancestor: u32) -> bool {
    let mut at = from;
    loop {
        if at == ancestor {
            return true;
        }
        match st.surfaces.get(&(c, at)).map(|s| &s.role) {
            Some(&Role::Subsurface { parent, .. }) => at = parent,
            _ => return false,
        }
    }
}

impl wl_subcompositor::Handler for State {
    fn get_subsurface(&mut self, cx: Cx, id: Obj, surface: u32, parent: u32) -> Res {
        use wl_subcompositor::error::{BAD_PARENT, BAD_SURFACE};
        let c = cx.client;
        if !self.surfaces.contains_key(&(c, parent)) {
            let message = format!("{parent} is not a surface");
            return Err(Error::new(DISPLAY.id, wl_display::error::INVALID_OBJECT, message));
        }
        cx.error_if(descends(self, c, parent, surface), BAD_PARENT, "parent is the surface or one of its descendants")?;
        self.client(c).set_data(id.id, surface);
        let role = Role::Subsurface { id: id.id, parent, x: 0, y: 0, sync: true };
        assign_role(self, (c, surface), role).map_err(|m| cx.error(BAD_SURFACE, m))?;
        self.surfaces.get_mut(&(c, parent)).expect("checked above").pending.added.push(surface);
        Ok(())
    }
}

fn subsurface(st: &mut State, cx: Cx) -> Option<(&mut bool, u32)> {
    match &mut st.surfaces.get_mut(&cx.surface())?.role {
        Role::Subsurface { id, parent, sync, .. } if *id == cx.obj.id => Some((sync, *parent)),
        _ => None,
    }
}

impl wl_subsurface::Handler for State {
    fn destroy(&mut self, cx: Cx) {
        let Some((_, parent)) = subsurface(self, cx) else { return };
        if let Some(s) = self.surfaces.get_mut(&cx.surface()) {
            s.role = Role::None;
        }
        detach(self, cx.surface(), parent);
        desynced(self, cx.surface());
    }

    fn set_position(&mut self, cx: Cx, x: i32, y: i32) {
        let Some((_, parent)) = subsurface(self, cx) else { return };
        if let Some(p) = self.surfaces.get_mut(&(cx.client, parent)) {
            p.pending.positions.push((cx.data, (x, y)));
        }
    }

    fn place_above(&mut self, cx: Cx, sibling: u32) -> Res {
        place(self, cx, sibling, 1)
    }

    fn place_below(&mut self, cx: Cx, sibling: u32) -> Res {
        place(self, cx, sibling, 0)
    }

    fn set_sync(&mut self, cx: Cx) {
        if let Some((sync, _)) = subsurface(self, cx) {
            *sync = true;
        }
    }

    fn set_desync(&mut self, cx: Cx) {
        let Some((sync, _)) = subsurface(self, cx) else { return };
        *sync = false;
        desynced(self, cx.surface());
    }
}

fn place(st: &mut State, cx: Cx, sibling: u32, above: usize) -> Res {
    let Some((_, parent)) = subsurface(st, cx) else { return Ok(()) };
    let (c, surface) = cx.surface();
    let sibling_of = |s: &Surface| matches!(s.role, Role::Subsurface { parent: p, .. } if p == parent);
    let valid = sibling != surface && (sibling == parent || st.surfaces.get(&(c, sibling)).is_some_and(sibling_of));
    let Some(p) = st.surfaces.get_mut(&(c, parent)).filter(|_| valid) else {
        return Err(cx.error(wl_subsurface::error::BAD_SURFACE, "not a sibling or the parent"));
    };
    let mut order = p.pending.order.take().unwrap_or_else(|| p.stack());
    order.retain(|&e| e != Some(surface));
    let target = (sibling != parent).then_some(sibling);
    let at = order.iter().position(|&e| e == target).map_or(order.len(), |at| at + above);
    order.insert(at, Some(surface));
    p.pending.order = Some(order);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn restack_tracks_the_parent_and_keeps_new_children_on_top() {
        let mut s = Surface::new(1);
        s.children = vec![2, 3, 4];
        restack(&mut s, vec![Some(4), None, Some(2), Some(9)]);
        assert_eq!((s.children.clone(), s.below), (vec![4, 2, 3], 1));
        assert_eq!(s.stack(), vec![Some(4), None, Some(2), Some(3)]);
    }

    #[test]
    fn damage_is_clamped() {
        let r = clamped(-1 << 40, 0, i64::from(i32::MAX) * 4, 5);
        assert_eq!(r, Rect::new(-MAX_DAMAGE as i32, 0, MAX_DAMAGE as i32, 5));
    }
}
