use crate::{
    Key, State,
    backend::Backend,
    base, client, cursor,
    output::{self, Damage},
    privileged::{capture, session_lock},
    scene::{self, Item},
    surface::{self, Role},
};
use backend::OutputId;
use gfx::{Destination, Op, Renderer, TexId};
use std::{
    collections::HashMap,
    error,
    os::fd::{AsFd, OwnedFd},
    rc::Rc,
    slice,
    sync::atomic::{AtomicU64, Ordering::Relaxed},
    time::Instant,
};
use wm::Rect;

pub fn render(st: &mut State) {
    let dead = st.garbage.take();
    for &(serial, texture) in &dead {
        texture.into_iter().chain(st.capture_targets.remove(&serial)).for_each(|t| st.renderer.free(t));
    }
    if let Backend::Kms(kms) = &mut st.backend {
        kms.forget_clients(dead.into_iter().map(|d| d.0));
    }
    gpu_done(st);
    capture::window_frames(st);
    if std::mem::take(&mut st.render_dirty) {
        st.outputs.iter_mut().for_each(|o| o.dirty = true);
    }
    let now = Instant::now();
    let due: Vec<OutputId> = st
        .outputs
        .iter()
        .filter(|o| o.enabled && !o.off && o.dirty && st.backend.ready(o.id))
        .filter(|o| o.render_at(tearing(st, o.id)).is_none_or(|t| t <= now))
        .map(|o| o.id)
        .collect();
    let dark: Vec<OutputId> = st
        .outputs
        .iter()
        .filter(|o| o.enabled && o.off && o.mirror.is_none() && capture::wants(st, o.id))
        .map(|o| o.id)
        .collect();
    if !dark.is_empty() {
        let (items, icon) = (scene::build(st), crate::dnd::icon(st));
        dark.into_iter().for_each(|id| capture_dark(st, id, &items, icon.as_ref()));
    }
    if due.is_empty() {
        return;
    }
    evict_texts(st);
    let items = scene::build(st);
    let icon = crate::dnd::icon(st);
    let presented: Vec<OutputId> =
        due.into_iter().filter(|&output| render_output(st, output, &items, icon.as_ref())).collect();
    let Some(&fallback) = presented.first() else { return };
    st.profile.all_latched();
    let mut discarded = vec![];
    for (key, surface) in &mut st.surfaces {
        let shown = surface.outputs.iter().copied().find(|o| presented.contains(o));
        let Some(output) = shown.or(surface.outputs.is_empty().then_some(fallback)) else { continue };
        st.presented.extend(surface.callbacks.drain(..).map(|id| ((key.0, id), output)));
        match shown {
            Some(_) => st.presented_feedback.extend(surface.feedbacks.drain(..).map(|id| ((key.0, id), output))),
            None => discarded.push((key.0, std::mem::take(&mut surface.feedbacks))),
        }
    }
    for (c, feedbacks) in discarded {
        surface::discard(st, c, feedbacks);
    }
    session_lock::presented(st);
    disconnect_faulted(st);
}

fn gpu_done(st: &mut State) {
    st.renderer.poll();
    for (seq, gpu) in st.renderer.gpu_times() {
        let Some(o) = st.outputs.iter_mut().find(|o| o.gpu_frame.is_some_and(|f| f.0 == seq)) else { continue };
        let Some((_, start)) = o.gpu_frame.take() else { continue };
        o.gpu_cost = gpu;
        st.profile.gpu(o.id, start, gpu);
    }
    surface::release_retired(st);
}

fn capture_dark(st: &mut State, id: OutputId, scene: &[Item], icon: Option<&Item>) {
    let Some(o) = output::find(st, id) else { return };
    let (size, view) = (o.frame().logical_size(), (o.rect, o.scale));
    let mut ops = Ops::new(Rect::new(0, 0, size.0, size.1));
    push_items(st, scene, view, &mut ops);
    push_items(st, icon.map(slice::from_ref).unwrap_or_default(), view, &mut ops);
    let split = ops.ops.len();
    let cursor = cursor::item(st, id);
    push_items(st, cursor.as_slice(), view, &mut ops);
    let (scene_ops, pointer) = ops.ops.split_at(split);
    capture::output_frames(st, id, size, (scene_ops, pointer), false, false);
}

fn render_output(st: &mut State, id: OutputId, scene: &[Item], icon: Option<&Item>) -> bool {
    let start = Instant::now();
    let Some(output) = output::find(st, id) else { return false };
    let (frame, origin, scale, mirror, cursor_shown) =
        (output.frame(), output.rect, output.scale, output.mirror, output.cursor_shown);
    let size = frame.logical_size();
    let plane_cursor = match &st.backend {
        Backend::Kms(kms) if mirror.is_none() => kms.cursor_size(id).and_then(|size| cursor_image(st, id, size)),
        _ => None,
    };
    let cursor_state = plane_cursor.as_ref().map(|c| (c.0, c.2));
    let cursor_changed = cursor_shown != cursor_state;
    let mut ops = Ops::new(Rect::new(0, 0, size.0, size.1));
    match mirror {
        Some(source) => mirror_ops(st, id, source, scene, icon, &mut ops),
        None => {
            push_items(st, scene, (origin, scale), &mut ops);
            push_items(st, icon.map(slice::from_ref).unwrap_or_default(), (origin, scale), &mut ops);
        }
    }
    let split = ops.ops.len();
    if mirror.is_none() {
        let cursor = cursor::item(st, id);
        push_items(st, cursor.as_slice(), (origin, scale), &mut ops);
    }
    let shown = if plane_cursor.is_some() { split } else { ops.ops.len() };
    let changed = output::find_mut(st, id).map_or(Rect::default(), |o| o.damage.update(&ops.contents[..shown]));
    let (scene_ops, pointer) = ops.ops.split_at(split);
    capture::output_frames(st, id, size, (scene_ops, pointer), !changed.is_empty(), cursor_changed);
    if changed.is_empty() && !waiting(st, id) && !cursor_changed && !st.backend.needs_commit(id) {
        if let Some(o) = output::find_mut(st, id) {
            o.dirty = false;
        }
        return false;
    }
    let candidate = scanout_candidate(&ops, shown, frame);
    let keep = changed.is_empty();
    let ops = &ops.ops[..shown];
    let tearing = tearing(st, id);
    let submitted = st.renderer.submitted();
    let damage = &mut st.outputs.iter_mut().find(|o| o.id == id).expect("rendered output exists").damage;
    let presented = match &mut st.backend {
        Backend::Nested(nested) => {
            let (renderer, targets) = (&mut st.renderer, &mut st.scanout_targets);
            let mut result = Ok(());
            let presented = nested.present(id, |target| {
                result = match target {
                    nested::Target::Memory(pixels) => {
                        renderer.render(ops, frame, Destination::Memory(pixels), None).map(drop)
                    }
                    nested::Target::Dmabuf(serial, dmabuf) => {
                        let fence = render_into(renderer, targets, (serial, dmabuf), (frame, ops, damage));
                        fence.map(|fence| fence.into_iter().for_each(|f| gfx::attach_fence(dmabuf, f.as_fd())))
                    }
                }
            });
            let live = nested.serials();
            damage.forget(&live);
            free_dead(renderer, targets, &live);
            result.and(presented).map(|p| if p { Shown::Composited } else { Shown::Nothing })
        }
        Backend::Kms(kms) => 'kms: {
            let cursor = plane_cursor.as_ref().map(|(key, pixels, position)| kms::CursorImage {
                key: *key,
                pixels: &pixels.data,
                size: (pixels.width, pixels.height),
                position: *position,
            });
            if let Some(buffer) = &candidate
                && let Some(dmabuf) = buffer.dmabuf().filter(|d| kms.can_scanout(id, buffer.serial, d))
            {
                match kms.present(id, kms::Primary::Client(buffer.serial, dmabuf), cursor, tearing) {
                    Ok(()) => break 'kms Ok(Shown::Scanout),
                    Err(e) => log::debug!("direct scanout refused, compositing: {e}"),
                }
            }
            if keep && kms.has_front(id) {
                break 'kms kms.present(id, kms::Primary::Keep, cursor, tearing).map(|()| Shown::Kept);
            }
            let Some((slot, scanout)) = kms.next_buffer(id) else { break 'kms Ok(Shown::Nothing) };
            let target = (scanout.serial, &scanout.dmabuf);
            match render_into(&mut st.renderer, &mut st.scanout_targets, target, (frame, ops, damage)) {
                Ok(fence) => {
                    kms.present(id, kms::Primary::Slot(slot, fence), cursor, tearing).map(|()| Shown::Composited)
                }
                Err(e) => Err(e),
            }
        }
    };
    let rendered = Some(st.renderer.submitted()).filter(|&s| s > submitted);
    match presented {
        Ok(shown) => {
            let presented = shown != Shown::Nothing;
            if presented {
                let name = st.outputs.iter().find(|o| o.id == id).map_or("", |o| o.info.name.as_str());
                st.profile.rendered(id, name, start, shown == Shown::Scanout);
            }
            if let Some(o) = output::find_mut(st, id) {
                o.dirty &= !presented;
                o.retry = None;
                if presented {
                    o.cursor_shown = cursor_state;
                    o.render_cost = (o.render_cost * 7 + start.elapsed() + o.gpu_cost) / 8;
                }
                if let Some(seq) = rendered.filter(|_| presented) {
                    o.gpu_frame = Some((seq, start));
                }
            }
            presented
        }
        Err(e) => {
            log::error!("cannot present on output {id}: {e}");
            if let Some(o) = output::find_mut(st, id) {
                o.retry = Some(Instant::now() + o.period());
            }
            false
        }
    }
}

fn push_items(st: &mut State, items: &[Item], (origin, scale): (Rect, f64), ops: &mut Ops) {
    let local = |r: Rect| Rect { x: r.x - origin.x, y: r.y - origin.y, ..r }.scaled(scale);
    for item in items {
        match *item {
            Item::Fill(rect, color) => ops.push(Op::Fill(local(rect), color), u64::from(color)),
            Item::Surface { key, origin: at, clip } => push_surface(st, key, at, local(clip), (origin, scale), ops),
            Item::Ghost { window, rect } => {
                let Some(buffer) = st.shell.ghosts.get(&window) else { continue };
                if let Some(tex) = buffer.texture(&mut st.renderer) {
                    ops.texture(tex, buffer.size(), local(rect), (1 << 62) | buffer.serial);
                }
            }
            Item::Image { ref image, dst } => {
                if let Some(tex) = image.texture(&mut st.renderer) {
                    ops.texture(tex, (image.pixels.width, image.pixels.height), dst, image.id);
                }
            }
            Item::Text { rect, ref text, ref font, fg, bg } => {
                let dst = local(rect);
                if let Some(Text { texture, id, .. }) = text_texture(st, (text, font, fg, bg), dst, scale) {
                    ops.texture(texture, (dst.w, dst.h), dst, id);
                }
            }
        }
    }
}

fn source_ops(st: &mut State, source: OutputId, scene: &[Item], icon: Option<&Item>) -> Option<(Ops, (i32, i32))> {
    let o = output::find(st, source)?;
    let (view, (w, h)) = ((o.rect, o.scale), o.frame().logical_size());
    let mut ops = Ops::new(Rect::new(0, 0, w, h));
    push_items(st, scene, view, &mut ops);
    push_items(st, icon.map(slice::from_ref).unwrap_or_default(), view, &mut ops);
    let cursor = cursor::item(st, source);
    push_items(st, cursor.as_slice(), view, &mut ops);
    Some((ops, (w, h)))
}

fn mirror_ops(st: &mut State, id: OutputId, source: OutputId, scene: &[Item], icon: Option<&Item>, ops: &mut Ops) {
    let Some(o) = output::find(st, id) else { return };
    let (target, frozen) = (o.frame().logical_size(), o.frozen.filter(|_| !session_lock::locked(st)));
    ops.push(Op::Fill(Rect::new(0, 0, target.0, target.1), 0xff000000), 0);
    let (shown, size) = match frozen {
        Some((tex, (w, h), content)) => {
            let mut frozen = Ops::new(Rect::new(0, 0, w, h));
            frozen.texture(tex, (w, h), Rect::new(0, 0, w, h), content);
            (frozen, (w, h))
        }
        None => match source_ops(st, source, scene, icon) {
            Some(found) => found,
            None => return,
        },
    };
    let scale = (f64::from(target.0) / f64::from(size.0)).min(f64::from(target.1) / f64::from(size.1));
    let fitted = Rect::new(0, 0, size.0, size.1).scaled(scale);
    let (dx, dy) = ((target.0 - fitted.w) / 2, (target.1 - fitted.h) / 2);
    let place = |r: Rect| {
        let r = r.scaled(scale);
        Rect { x: r.x + dx, y: r.y + dy, ..r }
    };
    for ((op, content), source) in shown.ops.into_iter().zip(shown.contents).zip(shown.sources) {
        let op = match op {
            Op::Fill(r, color) => Op::Fill(place(r), color),
            Op::Texture { tex, source, transform, dst, clip, alpha } => {
                Op::Texture { tex, source, transform, dst: place(dst), clip: place(clip), alpha }
            }
        };
        ops.push_from(op, content.1, source);
    }
}

pub fn freeze_mirrors(st: &mut State, freeze: bool) {
    let mirrors: Vec<(OutputId, OutputId)> =
        st.outputs.iter().filter_map(|o| Some((o.id, o.mirror?))).filter(|_| freeze).collect();
    for o in &mut st.outputs {
        if let Some((tex, ..)) = o.frozen.take() {
            st.renderer.free(tex);
        }
    }
    let scene = scene::build(st);
    for (id, source) in mirrors {
        let captured = source_ops(st, source, &scene, None).and_then(|(ops, (w, h))| {
            let mut pixels = vec![0; (w * h) as usize];
            let frame = gfx::Frame { size: (w, h), transform: 0 };
            st.renderer.render(&ops.ops, frame, Destination::Memory(&mut pixels), None).ok()?;
            let tex = st.renderer.upload(None, &gfx::Pixels { data: pixels, width: w, height: h }).ok()?;
            Some((tex, (w, h), content_id()))
        });
        if let Some(o) = output::find_mut(st, id) {
            o.frozen = captured;
        }
    }
    st.render_dirty = true;
}

pub fn window_ops(st: &mut State, key: Key, scale: f64) -> Option<(Vec<Op>, (i32, i32))> {
    let s = st.surfaces.get(&key)?;
    let g = Some(s.geometry).filter(|g| !g.is_empty()).unwrap_or_else(|| {
        let (w, h) = s.size();
        Rect::new(0, 0, w, h)
    });
    let size = g.scaled(scale);
    let bounds = Rect::new(0, 0, size.w, size.h);
    let mut ops = Ops::new(bounds);
    push_surface(st, key, (-g.x, -g.y), bounds, (Rect::default(), scale), &mut ops);
    Some((ops.ops, (size.w, size.h)))
}

pub fn draw_into(st: &mut State, ops: &[Op], (w, h): (i32, i32), buffer: &crate::buffer::Buffer) -> bool {
    let frame = gfx::Frame { size: (w, h), transform: 0 };
    if let Some(shm) = buffer.shm() {
        let mut pixels = vec![0; (w * h) as usize];
        let rendered = st.renderer.render(ops, frame, Destination::Memory(&mut pixels), None);
        return rendered.is_ok() && shm.write_from(&pixels, w as usize, 0, 0);
    }
    let Some(dmabuf) = buffer.dmabuf() else { return false };
    let target = match target(&mut st.renderer, &mut st.capture_targets, (buffer.serial, dmabuf)) {
        Ok(target) => target,
        Err(e) => {
            log::warn!("cannot capture into dmabuf: {e}");
            return false;
        }
    };
    match st.renderer.render(ops, frame, Destination::Texture(target), None) {
        Ok(fence) => {
            fence.into_iter().for_each(|f| gfx::attach_fence(dmabuf, f.as_fd()));
            true
        }
        Err(_) => false,
    }
}

pub fn content_id() -> u64 {
    static NEXT: AtomicU64 = AtomicU64::new(0);
    (1 << 63) | (NEXT.fetch_add(1, Relaxed) + 1)
}

struct Ops {
    bounds: Rect,
    ops: Vec<Op>,
    contents: Vec<(Rect, u64)>,
    sources: Vec<Option<Rc<crate::buffer::Buffer>>>,
}

impl Ops {
    fn new(bounds: Rect) -> Self {
        Self { bounds, ops: vec![], contents: vec![], sources: vec![] }
    }

    fn push(&mut self, op: Op, content: u64) {
        self.push_from(op, content, None);
    }

    fn texture(&mut self, tex: TexId, (w, h): (i32, i32), dst: Rect, content: u64) {
        let source = [0.0, 0.0, f64::from(w), f64::from(h)];
        self.push(Op::Texture { tex, source, transform: 0, dst, clip: dst, alpha: 1.0 }, content);
    }

    fn push_from(&mut self, op: Op, content: u64, source: Option<Rc<crate::buffer::Buffer>>) {
        let area = match op {
            Op::Fill(rect, _) => rect,
            Op::Texture { dst, clip, .. } => dst.intersect(clip),
        };
        let area = area.intersect(self.bounds);
        if area.is_empty() {
            return;
        }
        self.sources.push(source);
        self.ops.push(op);
        self.contents.push((area, content));
    }
}

fn waiting(st: &State, output: OutputId) -> bool {
    let mirror = output::find(st, output).is_some_and(|o| o.mirror.is_some());
    let on = |s: &crate::surface::Surface| !mirror && (s.outputs.is_empty() || s.outputs.contains(&output));
    let mut surfaces = st.surfaces.values().filter(|s| on(s));
    let barrier = st.surfaces.values().any(|s| s.barrier.iter().any(|b| b.0 == output));
    capture::wants(st, output) || barrier || surfaces.any(|s| !s.callbacks.is_empty() || !s.feedbacks.is_empty())
}

#[derive(Clone, Copy, PartialEq)]
enum Shown {
    Nothing,
    Composited,
    Scanout,
    Kept,
}

fn scanout_candidate(ops: &Ops, len: usize, frame: gfx::Frame) -> Option<Rc<crate::buffer::Buffer>> {
    let (op, buffer) = ops.ops[..len].last().zip(ops.sources[..len].last())?;
    let opaque = |b: &crate::buffer::Buffer| b.dmabuf().is_some_and(|d| d.format as u8 == b'X');
    let buffer = buffer.clone().filter(|b| opaque(b) && frame.transform == 0)?;
    let Op::Texture { dst, clip, alpha, source, transform: 0, .. } = *op else { return None };
    let full = Rect::new(0, 0, frame.size.0, frame.size.1);
    let (w, h) = buffer.size();
    let whole = source == [0.0, 0.0, f64::from(w), f64::from(h)];
    (dst == full && clip.intersect(dst) == full && alpha >= 1.0 && whole).then_some(buffer)
}

fn cursor_image(st: &mut State, id: OutputId, (cw, ch): (i32, i32)) -> Option<(u64, Rc<gfx::Pixels>, (i32, i32))> {
    let o = output::find(st, id).filter(|o| o.transform == 0)?;
    let (rect, scale) = (o.rect, o.scale);
    let fits = |w: i32, h: i32| w <= cw && h <= ch;
    match cursor::item(st, id)? {
        Item::Image { image, dst }
            if (dst.w, dst.h) == (image.pixels.width, image.pixels.height) && fits(dst.w, dst.h) =>
        {
            Some((image.id, image.pixels.clone(), (dst.x, dst.y)))
        }
        Item::Surface { key, origin, .. } => {
            let s = st.surfaces.get(&key).filter(|s| s.transform == 0)?;
            let buffer = s.buffer.clone()?;
            let ((w, h), (sw, sh)) = (buffer.size(), s.size());
            let physical = |v: i32| (f64::from(v) * scale).round() as i32;
            if (physical(sw), physical(sh)) != (w, h) || !fits(w, h) {
                return None;
            }
            let content = content_key(key, s.commits);
            let pixels = match &st.cursor_cache {
                Some((cached, pixels)) if *cached == content => pixels.clone(),
                _ => {
                    let shm = buffer.shm()?;
                    let mut data = vec![0; (w * h) as usize];
                    data.chunks_mut(w as usize).zip(0..).for_each(|(row, y)| gfx::Source::read_row(shm, 0, y, row));
                    let pixels = Rc::new(gfx::Pixels { data, width: w, height: h });
                    st.cursor_cache = Some((content, pixels.clone()));
                    pixels
                }
            };
            Some((content, pixels, (physical(origin.0 - rect.x), physical(origin.1 - rect.y))))
        }
        _ => None,
    }
}

fn render_into(
    renderer: &mut Renderer,
    targets: &mut HashMap<u64, TexId>,
    (serial, dmabuf): (u64, &gfx::Dmabuf),
    (frame, ops, damage): (gfx::Frame, &[Op], &mut Damage),
) -> Result<Option<OwnedFd>, Box<dyn error::Error>> {
    let target = target(renderer, targets, (serial, dmabuf))?;
    let fence = renderer.render(ops, frame, Destination::Texture(target), damage.region(serial))?;
    damage.drawn(serial);
    Ok(fence)
}

fn target(
    renderer: &mut Renderer,
    targets: &mut HashMap<u64, TexId>,
    (serial, dmabuf): (u64, &gfx::Dmabuf),
) -> Result<TexId, Box<dyn error::Error>> {
    if let Some(&target) = targets.get(&serial) {
        return Ok(target);
    }
    let target = renderer.import(dmabuf, true)?;
    targets.insert(serial, target);
    Ok(target)
}

fn free_dead(renderer: &mut Renderer, targets: &mut HashMap<u64, TexId>, live: &[u64]) {
    for (_, texture) in targets.extract_if(|serial, _| !live.contains(serial)) {
        renderer.free(texture);
    }
}

pub fn content_key(key: Key, commits: u64) -> u64 {
    (key.0 << 40) ^ (u64::from(key.1) << 20) ^ commits
}

pub type TextKey = (Rc<str>, Rc<str>, u32, u32, i32, i32);

#[derive(Clone, Copy)]
pub struct Text {
    texture: TexId,
    id: u64,
    used: bool,
}

fn text_texture(
    st: &mut State,
    (text, font, fg, bg): (&Rc<str>, &Rc<str>, u32, u32),
    dst: Rect,
    scale: f64,
) -> Option<Text> {
    let key = (text.clone(), font.clone(), fg, bg, dst.w, dst.h);
    if let Some(cached) = st.texts.get_mut(&key) {
        cached.used = true;
        return Some(*cached);
    }
    let pixels = gfx::text(text, font, fg, bg, (dst.w, dst.h), scale)?;
    let texture = st.renderer.upload(None, &pixels).inspect_err(|e| log::warn!("cannot upload text: {e}")).ok()?;
    let cached = Text { texture, id: content_id(), used: true };
    st.texts.insert(key, cached);
    Some(cached)
}

fn evict_texts(st: &mut State) {
    if st.texts.len() <= 256 {
        return;
    }
    let renderer = &mut st.renderer;
    st.texts.retain(|_, t| {
        let used = std::mem::take(&mut t.used);
        if !used {
            renderer.free(t.texture);
        }
        used
    });
}

fn push_surface(st: &mut State, key: Key, (x, y): (i32, i32), clip: Rect, view: (Rect, f64), ops: &mut Ops) {
    let Some(surface) = st.surfaces.get(&key) else { return };
    let ((w, h), source, transform, alpha) = (surface.size(), surface.source(), surface.transform, surface.alpha);
    let content = content_key(key, surface.commits);
    let (children, below) = (surface.children.clone(), surface.below.min(surface.children.len()));
    let buffer = surface.buffer.clone();
    let push_children = |st: &mut State, ops: &mut Ops, children: &[u32]| {
        for &child in children {
            let child = (key.0, child);
            if let Some(&Role::Subsurface { x: cx, y: cy, .. }) = st.surfaces.get(&child).map(|s| &s.role) {
                push_surface(st, child, (x + cx, y + cy), clip, view, ops);
            }
        }
    };
    push_children(st, ops, &children[..below]);
    let texture = buffer.as_ref().and_then(|buffer| buffer.texture(&mut st.renderer));
    let (origin, scale) = view;
    if let Some(tex) = texture {
        let f = |v: i32| (f64::from(v) * scale).round() as i32;
        let dst = Rect::new(f(x - origin.x), f(y - origin.y), f(w), f(h));
        ops.push_from(Op::Texture { tex, source, transform, dst, clip, alpha }, content, buffer);
    }
    push_children(st, ops, &children[below..]);
}

fn disconnect_faulted(st: &mut State) {
    let faulted: Vec<_> =
        st.surfaces.iter().filter(|(_, s)| s.buffer.as_ref().is_some_and(|b| b.faulted())).map(|(k, _)| k.0).collect();
    for c in faulted {
        client::fail(st, c, wire::Error::new(1, wire::wl_display::error::IMPLEMENTATION, "shm pool was truncated"));
    }
}

pub fn deadline(st: &State) -> Option<Instant> {
    let dirty = st.outputs.iter().filter(|o| o.enabled && !o.off && o.dirty && st.backend.ready(o.id));
    dirty.filter_map(|o| o.render_at(tearing(st, o.id))).min()
}

fn tearing(st: &State, output: OutputId) -> bool {
    let Some(rect) = output::find(st, output).map(|o| o.rect) else { return false };
    st.shell
        .shown
        .windows
        .iter()
        .filter(|p| p.fullscreen && p.rect.intersect(rect) == p.rect)
        .any(|p| st.windows.get(&p.window).and_then(|k| st.surfaces.get(k)).is_some_and(|s| s.tearing))
}

pub fn frame_done(st: &mut State, output: OutputId, flip: Option<backend::Flip>) {
    use wire::wp_presentation_feedback::{self as feedback, kind};
    gpu_done(st);
    st.profile.presented(output);
    let (now_sec, now_nsec) = crate::monotonic();
    let now = std::time::Duration::new(now_sec, now_nsec);
    let Some(o) = output::find_mut(st, output) else { return };
    o.frames += 1;
    if let Some(flip) = flip {
        let flipped = Instant::now() - now.saturating_sub(flip.time);
        o.next_vblank = Some(flipped + o.period());
    }
    let msc = flip.map_or(o.frames, |f| f.sequence);
    let (refresh, vrr) = (o.info.mode.refresh, o.vrr);
    let refresh = if refresh == 0 { 0 } else { (1_000_000_000_000 / u64::from(refresh)) as u32 };
    let pacing = match (refresh, vrr) {
        (0, _) => 0,
        (_, true) => kind::VARIABLE_RATE,
        (_, false) => kind::FIXED_RATE,
    };
    let hardware = match flip {
        Some(_) if tearing(st, output) => kind::HW_CLOCK | kind::HW_COMPLETION,
        Some(_) => kind::VSYNC | kind::HW_CLOCK | kind::HW_COMPLETION,
        None => 0,
    };
    let time = st.time();
    let done: Vec<_> = st.presented.extract_if(.., |p| p.1 == output).collect();
    for ((c, callback), _) in done {
        base::callback_done(st, c, callback, time);
    }
    let presented = flip.map_or(now, |f| f.time);
    let (sec, nsec) = (presented.as_secs(), presented.subsec_nanos());
    for ((c, id), _) in st.presented_feedback.extract_if(.., |p| p.1 == output) {
        let Some(client) = st.clients.get_mut(&c) else { continue };
        let Some(fb) = client.get(id).map(|o| o.obj) else { continue };
        for (wl_output, _) in client.outputs().into_iter().filter(|o| o.1 == Some(output)) {
            feedback::sync_output(&mut client.out, fb, wl_output.id);
        }
        let flags = hardware | if fb.version >= 3 { pacing } else { 0 };
        let ((hi, lo), (msc_hi, msc_lo)) = (((sec >> 32) as u32, sec as u32), ((msc >> 32) as u32, msc as u32));
        feedback::presented(&mut client.out, fb, hi, lo, nsec, refresh, msc_hi, msc_lo, flags);
        client.delete(id);
    }
    crate::commit::presented(st, output);
}
