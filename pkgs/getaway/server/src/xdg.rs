use crate::{
    ClientId, Key, State,
    buffer::Buffer,
    client::Data,
    dnd,
    privileged::capture,
    scene, seat, shell,
    surface::{self, Role, configure::Configures},
};
use std::{mem, rc::Rc};
use wire::*;
use wm::{Cause, OutputId, Placed, Rect, WindowId, WindowInfo};

const LIMIT: i32 = 1 << 24;

pub struct Toplevel {
    window: WindowId,
    xdg_surface: Obj,
    toplevel: Obj,
    info: WindowInfo,
    managed: Option<u64>,
    withdrawn: bool,
    fullscreen: Option<Option<OutputId>>,
    configures: Configures<Sent>,
    reconfigure: bool,
    decoration: Option<u32>,
}

#[derive(Clone, Copy, Default, PartialEq)]
struct Sent {
    size: (i32, i32),
    bounds: (i32, i32),
    activated: bool,
    fullscreen: bool,
    suspended: bool,
}

impl Toplevel {
    pub fn window(&self) -> WindowId {
        self.window
    }
}

pub struct Popup {
    pub parent: Option<u32>,
    xdg_surface: Obj,
    popup: Obj,
    positioner: Positioner,
    pub rect: Rect,
    pub grab: bool,
    configures: Configures<Rect>,
}

#[derive(Clone, Copy, Default)]
pub struct Positioner {
    size: (i32, i32),
    anchor_rect: Rect,
    anchor: u32,
    gravity: u32,
    offset: (i32, i32),
    adjust: u32,
}

impl Positioner {
    fn complete(&self) -> bool {
        self.size.0 > 0 && !self.anchor_rect.is_empty()
    }

    fn anchor_point(&self) -> (i32, i32) {
        let ((ax, ay), a) = (edges(self.anchor), self.anchor_rect);
        (a.x + a.w * (ax + 1) / 2, a.y + a.h * (ay + 1) / 2)
    }

    fn place(&self) -> Rect {
        let ((ax, ay), (gx, gy), (w, h)) = (self.anchor_point(), edges(self.gravity), self.size);
        let (x, y) = (ax - w * (1 - gx) / 2, ay - h * (1 - gy) / 2);
        Rect::new(x + self.offset.0, y + self.offset.1, w, h)
    }
}

fn flip(value: u32, horizontal: bool) -> u32 {
    let pairs: &[(u32, u32)] = if horizontal { &[(3, 4), (5, 7), (6, 8)] } else { &[(1, 2), (5, 6), (7, 8)] };
    pairs.iter().find_map(|&(a, b)| (value == a).then_some(b).or((value == b).then_some(a))).unwrap_or(value)
}

fn edges(value: u32) -> (i32, i32) {
    match value {
        1 => (0, -1),
        2 => (0, 1),
        3 => (-1, 0),
        4 => (1, 0),
        5 => (-1, -1),
        6 => (-1, 1),
        7 => (1, -1),
        8 => (1, 1),
        _ => (0, 0),
    }
}

impl xdg_wm_base::Handler for State {
    fn destroy(&mut self, cx: Cx) -> Res {
        let client = self.client(cx.client);
        if client.of(Interface::XdgWmBase).count() == 1 && client.of(Interface::XdgSurface).count() > 0 {
            return Err(cx.error(xdg_wm_base::error::DEFUNCT_SURFACES, "xdg_wm_base destroyed before its surfaces"));
        }
        Ok(())
    }

    fn create_positioner(&mut self, cx: Cx, id: Obj) {
        self.set((cx.client, id.id), Data::Positioner(Positioner::default()));
    }

    fn get_xdg_surface(&mut self, cx: Cx, id: Obj, surface: u32) {
        self.client(cx.client).set_data(id.id, surface);
    }

    fn pong(&mut self, _: Cx, _: u32) {}
}

fn clamp(v: i32) -> i32 {
    v.clamp(-LIMIT, LIMIT)
}

fn positioner(st: &mut State, cx: Cx) -> &mut Positioner {
    match st.data_mut(cx.key()) {
        Some(Data::Positioner(p)) => p,
        _ => unreachable!("positioner object has state"),
    }
}

impl xdg_positioner::Handler for State {
    fn set_size(&mut self, cx: Cx, width: i32, height: i32) -> Res {
        cx.error_if(width <= 0 || height <= 0, xdg_positioner::error::INVALID_INPUT, "invalid size")?;
        positioner(self, cx).size = (clamp(width), clamp(height));
        Ok(())
    }

    fn set_anchor_rect(&mut self, cx: Cx, x: i32, y: i32, width: i32, height: i32) -> Res {
        cx.error_if(width < 0 || height < 0, xdg_positioner::error::INVALID_INPUT, "invalid size")?;
        positioner(self, cx).anchor_rect = Rect::new(clamp(x), clamp(y), clamp(width), clamp(height));
        Ok(())
    }

    fn set_anchor(&mut self, cx: Cx, anchor: u32) {
        positioner(self, cx).anchor = anchor;
    }

    fn set_gravity(&mut self, cx: Cx, gravity: u32) {
        positioner(self, cx).gravity = gravity;
    }

    fn set_constraint_adjustment(&mut self, cx: Cx, constraint_adjustment: u32) {
        positioner(self, cx).adjust = constraint_adjustment;
    }

    fn set_offset(&mut self, cx: Cx, x: i32, y: i32) {
        positioner(self, cx).offset = (clamp(x), clamp(y));
    }

    fn set_reactive(&mut self, _: Cx) {}

    fn set_parent_size(&mut self, _: Cx, _: i32, _: i32) {}

    fn set_parent_configure(&mut self, _: Cx, _: u32) {}
}

impl xdg_surface::Handler for State {
    fn destroy(&mut self, cx: Cx) -> Res {
        let live = match self.surfaces.get(&cx.surface()).map(|s| &s.role) {
            Some(Role::Toplevel(t)) => t.xdg_surface == cx.obj,
            Some(Role::Popup(p)) => p.xdg_surface == cx.obj,
            _ => false,
        };
        if live {
            let defunct = xdg_surface::error::DEFUNCT_ROLE_OBJECT;
            return Err(cx.error(defunct, "xdg_surface destroyed before its role object"));
        }
        Ok(())
    }

    fn get_toplevel(&mut self, cx: Cx, toplevel: Obj) -> Res {
        let window = WindowId(self.next_id());
        let info = WindowInfo::default();
        let t = Toplevel {
            window,
            xdg_surface: cx.obj,
            toplevel,
            info,
            managed: None,
            withdrawn: false,
            fullscreen: None,
            configures: Configures::default(),
            reconfigure: false,
            decoration: None,
        };
        let capabilities = xdg_toplevel::wm_capabilities::FULLSCREEN.to_ne_bytes();
        self.send(cx.client, |out| xdg_toplevel::wm_capabilities(out, toplevel, &capabilities));
        surface::assign_role(self, cx.surface(), Role::Toplevel(t)).map_err(|m| role_error(cx, m))?;
        self.windows.insert(window, cx.surface());
        Ok(())
    }

    fn get_popup(&mut self, cx: Cx, popup: Obj, parent: Option<u32>, positioner: u32) -> Res {
        let parent = match parent.map(|p| self.client(cx.client).get_of(p, Interface::XdgSurface)) {
            None => None,
            Some(Some(s)) => Some(s.data),
            Some(None) => return Err(cx.error(xdg_wm_base::error::INVALID_POPUP_PARENT, "bad parent")),
        };
        let positioner = complete_positioner(self, cx, positioner)?;
        let p = Popup {
            parent,
            xdg_surface: cx.obj,
            popup,
            positioner,
            rect: Rect::default(),
            grab: false,
            configures: Configures::default(),
        };
        surface::assign_role(self, cx.surface(), Role::Popup(p)).map_err(|m| role_error(cx, m))?;
        self.popups.push(cx.surface());
        Ok(())
    }

    fn set_window_geometry(&mut self, cx: Cx, x: i32, y: i32, width: i32, height: i32) -> Res {
        cx.error_if(width <= 0 || height <= 0, xdg_surface::error::INVALID_SIZE, "invalid window geometry")?;
        if let Some(s) = self.surfaces.get_mut(&cx.surface()) {
            s.pending.state.geometry = Some(Rect::new(clamp(x), clamp(y), clamp(width), clamp(height)));
        }
        Ok(())
    }

    fn ack_configure(&mut self, cx: Cx, serial: u32) -> Res {
        let valid = match self.surfaces.get_mut(&cx.surface()).map(|s| &mut s.role) {
            Some(Role::Toplevel(t)) => t.configures.ack(serial),
            Some(Role::Popup(p)) => p.configures.ack(serial),
            _ => false,
        };
        cx.error_if(!valid, xdg_surface::error::INVALID_SERIAL, "invalid configure serial")
    }
}

fn role_error(cx: Cx, message: &str) -> Error {
    cx.error(xdg_wm_base::error::ROLE, message)
}

pub fn check_commit(st: &State, key: Key, buffer: bool) -> Result<Option<u32>, Error> {
    let (obj, acked) = match st.surfaces.get(&key).map(|s| &s.role) {
        Some(Role::Toplevel(t)) => (t.xdg_surface, t.configures.acked()),
        Some(Role::Popup(p)) => (p.xdg_surface, p.configures.acked()),
        _ => return Ok(None),
    };
    match acked {
        None if buffer => Err(Error::new(obj.id, xdg_surface::error::UNCONFIGURED_BUFFER, "buffer before first ack")),
        acked => Ok(acked),
    }
}

fn complete_positioner(st: &State, cx: Cx, positioner: u32) -> Result<Positioner, Error> {
    let positioner = match st.data((cx.client, positioner)) {
        Some(&Data::Positioner(p)) => Some(p).filter(Positioner::complete),
        _ => None,
    };
    positioner.ok_or_else(|| cx.error(xdg_wm_base::error::INVALID_POSITIONER, "incomplete positioner"))
}

impl xdg_toplevel::Handler for State {
    fn destroy(&mut self, cx: Cx) -> Res {
        if let Some(decoration) = toplevel_mut(self, cx.surface()).and_then(|t| t.decoration) {
            let orphaned = zxdg_toplevel_decoration_v1::error::ORPHANED;
            return Err(Error::new(decoration, orphaned, "toplevel destroyed before its decoration"));
        }
        unmap(self, cx.surface());
        Ok(())
    }

    fn set_parent(&mut self, cx: Cx, parent: Option<u32>) -> Res {
        let parent = parent.and_then(|p| toplevel_object(self, cx.client, p)).and_then(|k| window_of(self, k));
        let own = window_of(self, cx.surface());
        if parent.is_some_and(|p| ancestry(self, p).any(|a| Some(a) == own)) {
            return Err(cx.error(xdg_toplevel::error::INVALID_PARENT, "parent would create a loop"));
        }
        update_info(self, cx.surface(), |i| i.parent = parent);
        Ok(())
    }

    fn set_title(&mut self, cx: Cx, title: String) {
        update_info(self, cx.surface(), |i| i.title = title);
    }

    fn set_app_id(&mut self, cx: Cx, app_id: String) {
        update_info(self, cx.surface(), |i| i.app_id = app_id);
    }

    fn show_window_menu(&mut self, _: Cx, _: u32, _: u32, _: i32, _: i32) {}

    fn r#move(&mut self, _: Cx, _: u32, _: u32) {}

    fn resize(&mut self, _: Cx, _: u32, _: u32, _: u32) {}

    fn set_max_size(&mut self, _: Cx, _: i32, _: i32) {}

    fn set_min_size(&mut self, _: Cx, _: i32, _: i32) {}

    fn set_maximized(&mut self, _: Cx) {}

    fn unset_maximized(&mut self, _: Cx) {}

    fn set_fullscreen(&mut self, cx: Cx, output: Option<u32>) {
        let output = output.and_then(|o| self.client(cx.client).output(o));
        fullscreen(self, cx.surface(), Some(output));
    }

    fn unset_fullscreen(&mut self, cx: Cx) {
        fullscreen(self, cx.surface(), None);
    }

    fn set_minimized(&mut self, _: Cx) {}
}

pub fn toplevel_object(st: &mut State, c: ClientId, id: u32) -> Option<Key> {
    st.client(c).get_of(id, Interface::XdgToplevel).map(|o| (c, o.data))
}

impl xdg_popup::Handler for State {
    fn destroy(&mut self, cx: Cx) -> Res {
        if !descendants(self, cx.surface()).is_empty() {
            let error = xdg_wm_base::error::NOT_THE_TOPMOST_POPUP;
            return Err(cx.error(error, "destroyed popup is not the topmost popup"));
        }
        unmap(self, cx.surface());
        Ok(())
    }

    fn grab(&mut self, cx: Cx, _: u32, serial: u32) -> Res {
        let key = cx.surface();
        if self.surfaces.get(&key).is_some_and(|s| s.buffer.is_some()) {
            return Err(cx.error(xdg_popup::error::INVALID_GRAB, "grab after the popup was mapped"));
        }
        let allowed = seat::recent_input(self, cx.client, serial) && grab_allowed(self, key);
        let Some(p) = popup_mut(self, key) else { return Ok(()) };
        p.grab = allowed;
        if !allowed {
            let popup = p.popup;
            self.popups.retain(|&k| k != key);
            self.send(cx.client, |out| xdg_popup::popup_done(out, popup));
        }
        Ok(())
    }

    fn reposition(&mut self, cx: Cx, positioner: u32, token: u32) -> Res {
        let positioner = complete_positioner(self, cx, positioner)?;
        let Some(p) = popup_mut(self, cx.surface()) else { return Ok(()) };
        p.positioner = positioner;
        let obj = p.popup;
        self.send(cx.client, |out| xdg_popup::repositioned(out, obj, token));
        configure_popup(self, cx.surface());
        Ok(())
    }
}

impl zxdg_decoration_manager_v1::Handler for State {
    fn get_toplevel_decoration(&mut self, cx: Cx, decoration: Obj, toplevel: u32) -> Res {
        let key = toplevel_object(self, cx.client, toplevel);
        if let Some(t) = key.and_then(|k| toplevel_mut(self, k))
            && t.decoration.replace(decoration.id).is_some()
        {
            let constructed = zxdg_toplevel_decoration_v1::error::ALREADY_CONSTRUCTED;
            return Err(Error::new(decoration.id, constructed, "toplevel already has a decoration"));
        }
        key.into_iter().for_each(|k| self.client(cx.client).set_data(decoration.id, k.1));
        server_side(self, cx.client, decoration, key);
        Ok(())
    }
}

fn decorated(st: &State, cx: Cx) -> Option<Key> {
    Some(cx.surface()).filter(|&k| toplevel_of(st, k).is_some_and(|t| t.decoration == Some(cx.obj.id)))
}

impl zxdg_toplevel_decoration_v1::Handler for State {
    fn destroy(&mut self, cx: Cx) {
        if let Some(t) = decorated(self, cx).and_then(|k| toplevel_mut(self, k)) {
            t.decoration = None;
        }
    }

    fn set_mode(&mut self, cx: Cx, mode: u32) -> Res {
        use zxdg_toplevel_decoration_v1::{error, mode};
        if ![mode::CLIENT_SIDE, mode::SERVER_SIDE].contains(&mode) {
            return Err(cx.error(error::INVALID_MODE, "invalid decoration mode"));
        }
        server_side(self, cx.client, cx.obj, decorated(self, cx));
        Ok(())
    }

    fn unset_mode(&mut self, cx: Cx) {
        server_side(self, cx.client, cx.obj, decorated(self, cx));
    }
}

fn server_side(st: &mut State, c: ClientId, decoration: Obj, toplevel: Option<Key>) {
    let mode = zxdg_toplevel_decoration_v1::mode::SERVER_SIDE;
    st.send(c, |out| zxdg_toplevel_decoration_v1::configure(out, decoration, mode));
    if let Some(key) = toplevel {
        reconfigure(st, key);
    }
}

fn reconfigure(st: &mut State, key: Key) {
    let Some(t) = toplevel_mut(st, key).filter(|t| t.configures.last().is_some()) else { return };
    t.reconfigure = true;
    let window = t.window;
    send_configure(st, window, |sent| sent);
}

fn toplevel_mut(st: &mut State, key: Key) -> Option<&mut Toplevel> {
    match &mut st.surfaces.get_mut(&key)?.role {
        Role::Toplevel(t) => Some(t),
        _ => None,
    }
}

fn popup_mut(st: &mut State, key: Key) -> Option<&mut Popup> {
    match &mut st.surfaces.get_mut(&key)?.role {
        Role::Popup(p) => Some(p),
        _ => None,
    }
}

fn grab_allowed(st: &State, key: Key) -> bool {
    let Some(Role::Popup(p)) = st.surfaces.get(&key).map(|s| &s.role) else { return false };
    let Some(parent) = p.parent.map(|parent| (key.0, parent)) else { return false };
    seat::grabbing_popup(st) == Some(parent)
        || match st.surfaces.get(&parent).map(|s| &s.role) {
            Some(Role::Toplevel(t)) => st.shell.target.focus == Some(t.window),
            Some(Role::Popup(_)) => grab_allowed(st, parent),
            Some(Role::Layer(_)) => true,
            _ => false,
        }
}

fn update_info(st: &mut State, key: Key, f: impl FnOnce(&mut WindowInfo)) {
    let Some(t) = toplevel_mut(st, key) else { return };
    f(&mut t.info);
    let Some(window) = t.managed.map(|_| t.window) else { return };
    let Some(Role::Toplevel(t)) = st.surfaces.get(&key).map(|s| &s.role) else { return };
    st.shell.dirty |= st.policy.wm().window_changed(window, &t.info);
    st.shell.resync = true;
}

pub fn set_tag(st: &mut State, key: Key, tag: String) {
    update_info(st, key, |i| i.tag = tag);
}

fn fullscreen(st: &mut State, key: Key, request: Option<Option<OutputId>>) {
    let Some(t) = toplevel_mut(st, key) else { return };
    (t.fullscreen, t.reconfigure) = (request, true);
    if let Some(window) = t.managed.map(|_| t.window) {
        st.policy.wm().fullscreen(window, request.is_some(), request.flatten());
        st.shell.dirty = true;
    }
}

pub fn activate(st: &mut State, w: WindowId, trusted: bool) {
    if managed_windows(st).any(|m| m.0 == w) {
        st.policy.wm().focus(w, if trusted { Cause::Activation } else { Cause::Client });
        st.shell.dirty = true;
    }
}

pub fn committed(st: &mut State, key: Key) {
    let unmapped = st.surfaces.get(&key).is_some_and(|s| s.buffer.is_none());
    if unmapped && popup_mut(st, key).is_some_and(|p| p.configures.acked().is_some()) {
        dismiss_children(st, key);
        st.popups.retain(|&k| k != key);
        st.render_dirty = true;
        return;
    }
    let Some(s) = st.surfaces.get_mut(&key) else { return };
    match &mut s.role {
        Role::Toplevel(t) if t.withdrawn => t.withdrawn = false,
        Role::Toplevel(t) if t.managed.is_none() => {
            st.next_id += 1;
            t.managed = Some(st.next_id);
            let (window, info, fullscreen) = (t.window, t.info.clone(), t.fullscreen);
            st.policy.wm().window_added(window, &info);
            if let Some(output) = fullscreen {
                st.policy.wm().fullscreen(window, true, output);
            }
            st.shell.dirty = true;
        }
        Role::Popup(p) if p.configures.last().is_none() && p.parent.is_some() => configure_popup(st, key),
        Role::Popup(p) => p.rect = s.state.configure.and_then(|c| p.configures.get(c)).unwrap_or(p.rect),
        _ => {}
    }
}

pub fn configure_popup(st: &mut State, key: Key) {
    use xdg_positioner::constraint_adjustment::*;
    let Some((parent, positioner)) = popup_mut(st, key).map(|p| (p.parent, p.positioner)) else { return };
    let origin = parent.and_then(|parent| scene::geometry_origin(st, (key.0, parent))).unwrap_or_default();
    let place = |p: &Positioner| {
        let r = p.place();
        Rect { x: r.x.saturating_add(origin.0), y: r.y.saturating_add(origin.1), ..r }
    };
    let mut global = place(&positioner);
    let (ax, ay) = positioner.anchor_point();
    let anchor = Rect::new(ax.saturating_add(origin.0), ay.saturating_add(origin.1), 0, 0);
    let bounds = crate::output::containing(st, anchor).map_or(global, |o| o.rect);
    let fits = |r: Rect, horizontal: bool| match horizontal {
        true => r.x >= bounds.x && r.x.saturating_add(r.w) <= bounds.right(),
        false => r.y >= bounds.y && r.y.saturating_add(r.h) <= bounds.bottom(),
    };
    let mut current = positioner;
    for (horizontal, flag) in [(true, FLIP_X), (false, FLIP_Y)] {
        if current.adjust & flag == 0 || fits(global, horizontal) {
            continue;
        }
        let flipped = Positioner {
            anchor: flip(current.anchor, horizontal),
            gravity: flip(current.gravity, horizontal),
            ..current
        };
        if fits(place(&flipped), horizontal) {
            (current, global) = (flipped, place(&flipped));
        }
    }
    let (gx, gy) = edges(current.gravity);
    let adjust = |flag: u32| current.adjust & flag != 0;
    let (x, w) = unconstrain((global.x, global.w), (bounds.x, bounds.right()), gx, adjust(SLIDE_X), adjust(RESIZE_X));
    let (y, h) = unconstrain((global.y, global.h), (bounds.y, bounds.bottom()), gy, adjust(SLIDE_Y), adjust(RESIZE_Y));
    let rect = Rect::new(x.saturating_sub(origin.0), y.saturating_sub(origin.1), w, h);
    let serial = st.next_serial();
    let Some(p) = popup_mut(st, key) else { return };
    p.configures.send(serial, rect);
    let (popup, xdg_surface) = (p.popup, p.xdg_surface);
    st.send(key.0, |out| {
        xdg_popup::configure(out, popup, rect.x, rect.y, rect.w, rect.h);
        xdg_surface::configure(out, xdg_surface, serial);
    });
}

fn unconstrain((pos, size): (i32, i32), (lo, hi): (i32, i32), gravity: i32, slide: bool, resize: bool) -> (i32, i32) {
    let forward = |p: i32| p + (lo - p).min(hi - p - size).max(0);
    let backward = |p: i32| p - (p + size - hi).min(p - lo).max(0);
    let pos = match (slide, gravity < 0) {
        (false, _) => pos,
        (true, false) => backward(forward(pos)),
        (true, true) => forward(backward(pos)),
    };
    let (start, end) = (pos.max(lo), (pos + size).min(hi));
    match resize && end > start {
        true => (start, end - start),
        false => (pos, size),
    }
}

pub fn dismiss_popups(st: &mut State) {
    for key in mem::take(&mut st.popups).into_iter().rev() {
        if let Some(popup) = popup_mut(st, key).map(|p| p.popup) {
            st.send(key.0, |out| xdg_popup::popup_done(out, popup));
        }
    }
    st.render_dirty = true;
}

fn descendants(st: &State, key: Key) -> Vec<Key> {
    let parent = |k: &Key| match st.surfaces.get(k).map(|s| &s.role) {
        Some(Role::Popup(p)) => p.parent.map(|parent| (k.0, parent)),
        _ => None,
    };
    let mut family = vec![key];
    for k in &st.popups {
        if parent(k).is_some_and(|p| family.contains(&p)) {
            family.push(*k);
        }
    }
    family.split_off(1)
}

pub fn dismiss_children(st: &mut State, key: Key) {
    let children = descendants(st, key);
    for &child in children.iter().rev() {
        if let Some(popup) = popup_mut(st, child).map(|p| p.popup) {
            st.send(child.0, |out| xdg_popup::popup_done(out, popup));
        }
    }
    st.popups.retain(|k| !children.contains(k));
    st.render_dirty |= !children.is_empty();
}

pub fn unmap(st: &mut State, key: Key) {
    dismiss_children(st, key);
    let Some(surface) = st.surfaces.get_mut(&key) else { return };
    if let Role::Popup(_) = surface.role {
        surface.role = Role::None;
        st.popups.retain(|&k| k != key);
        st.render_dirty = true;
        return;
    }
    let Some(t) = surface.role.take_toplevel() else { return };
    let buffer = surface.buffer.clone();
    st.windows.remove(&t.window);
    dnd::toplevel_gone(st, key);
    if t.managed.is_some() {
        forget(st, t.window, buffer);
        capture::source_gone(st, capture::Source::Window(t.window));
    }
}

pub fn withdraw(st: &mut State, w: WindowId, buffer: Option<Rc<Buffer>>) {
    let Some(t) = st.windows.get(&w).copied().and_then(|k| toplevel_mut(st, k)).filter(|t| t.managed.is_some()) else {
        return;
    };
    (t.managed, t.withdrawn) = (None, true);
    t.configures.unmap();
    if let Some(&key) = st.windows.get(&w) {
        dismiss_children(st, key);
        dnd::toplevel_gone(st, key);
    }
    forget(st, w, buffer);
}

fn ancestry(st: &State, w: WindowId) -> impl Iterator<Item = WindowId> + '_ {
    let parent = |w: &WindowId| st.windows.get(w).and_then(|k| toplevel_of(st, *k)).and_then(|t| t.info.parent);
    std::iter::successors(Some(w), parent).take(64)
}

fn toplevel_of(st: &State, key: Key) -> Option<&Toplevel> {
    match &st.surfaces.get(&key)?.role {
        Role::Toplevel(t) => Some(t),
        _ => None,
    }
}

fn forget(st: &mut State, w: WindowId, buffer: Option<Rc<Buffer>>) {
    let grandparent = st.windows.get(&w).and_then(|&k| toplevel_of(st, k)).and_then(|t| t.info.parent);
    for surface in st.surfaces.values_mut() {
        if let Role::Toplevel(t) = &mut surface.role
            && t.info.parent == Some(w)
        {
            t.info.parent = grandparent;
        }
    }
    st.policy.wm().window_removed(w);
    st.shell.dirty = true;
    shell::window_gone(st, w, buffer);
    capture::toplevel_closed(st, w);
}

pub fn window_of(st: &State, key: Key) -> Option<WindowId> {
    toplevel_of(st, key).map(Toplevel::window)
}

pub fn managed_windows(st: &State) -> impl Iterator<Item = (WindowId, u64, &WindowInfo)> + '_ {
    st.windows.values().filter_map(|&k| toplevel_of(st, k)).filter_map(|t| Some((t.window, t.managed?, &t.info)))
}

pub fn close(st: &mut State, w: WindowId) {
    let Some(&key) = st.windows.get(&w) else { return };
    let Some(t) = toplevel_mut(st, key) else { return };
    let toplevel = t.toplevel;
    st.send(key.0, |out| xdg_toplevel::close(out, toplevel));
}

pub fn configure(st: &mut State, placed: &Placed) -> Option<u32> {
    let bounds = crate::output::containing(st, placed.rect).map_or((0, 0), |o| (o.usable.w, o.usable.h));
    let (size, activated, fullscreen) = ((placed.rect.w, placed.rect.h), placed.activated, placed.fullscreen);
    send_configure(st, placed.window, |_| Sent { size, bounds, activated, fullscreen, suspended: false })
}

pub fn suspend(st: &mut State, window: WindowId) {
    send_configure(st, window, |sent| Sent { activated: false, suspended: true, ..sent });
}

fn send_configure(st: &mut State, window: WindowId, state: impl FnOnce(Sent) -> Sent) -> Option<u32> {
    let key = *st.windows.get(&window)?;
    let serial = st.serial.wrapping_add(1);
    let t = toplevel_mut(st, key)?;
    let old = t.configures.last();
    let sent = state(old.unwrap_or_default());
    if !mem::take(&mut t.reconfigure) && old == Some(sent) {
        return None;
    }
    if old.is_some_and(|old| old.fullscreen != sent.fullscreen) {
        t.fullscreen = sent.fullscreen.then_some(t.fullscreen.flatten());
    }
    let resized = old.is_none_or(|old| old.size != sent.size);
    t.configures.send(serial, sent);
    let (toplevel, xdg_surface) = (t.toplevel, t.xdg_surface);
    st.serial = serial;
    let states = toplevel_states(toplevel.version, sent);
    st.send(key.0, |out| {
        xdg_toplevel::configure_bounds(out, toplevel, sent.bounds.0, sent.bounds.1);
        xdg_toplevel::configure(out, toplevel, sent.size.0, sent.size.1, &states);
        xdg_surface::configure(out, xdg_surface, serial);
    });
    (resized && !sent.suspended).then_some(serial)
}

fn toplevel_states(version: u32, sent: Sent) -> Vec<u8> {
    use xdg_toplevel::state::*;
    let mut states = vec![];
    if sent.activated {
        states.push(ACTIVATED);
    }
    match sent.fullscreen {
        true => states.push(FULLSCREEN),
        false => states.extend([TILED_LEFT, TILED_RIGHT, TILED_TOP, TILED_BOTTOM]),
    }
    states.extend([CONSTRAINED_LEFT, CONSTRAINED_RIGHT, CONSTRAINED_TOP, CONSTRAINED_BOTTOM]);
    if sent.suspended {
        states.push(SUSPENDED);
    }
    let since = |s: u32| match s {
        TILED_LEFT..=TILED_BOTTOM => 2,
        SUSPENDED => 6,
        CONSTRAINED_LEFT.. => 7,
        _ => 1,
    };
    states.into_iter().filter(|&s| version >= since(s)).flat_map(|s| s.to_ne_bytes()).collect()
}
