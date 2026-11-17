use crate::{
    Key, State,
    surface::{self, Role, configure::Configures, double_buffered},
};
use wire::{zwlr_layer_surface_v1::anchor, *};
use wm::Rect;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum LayerKind {
    Background,
    Bottom,
    Top,
    Overlay,
}

double_buffered! {
    LayerChanges => LayerState {
        size: (u32, u32) = (0, 0),
        anchor: u32 = 0,
        zone: i32 = 0,
        margin: [i32; 4] = [0; 4],
        keyboard: u32 = 0,
        layer: u32 = 0,
    }
}

pub struct Layer {
    obj: Obj,
    output: backend::OutputId,
    current: LayerState,
    pub rect: Rect,
    configures: Configures<(i32, i32)>,
    committed: bool,
    mapped: bool,
}

impl Layer {
    pub fn kind(&self) -> LayerKind {
        match self.current.layer {
            0 => LayerKind::Background,
            1 => LayerKind::Bottom,
            2 => LayerKind::Top,
            _ => LayerKind::Overlay,
        }
    }

    pub fn wants_keyboard(&self, on_demand: bool) -> bool {
        use zwlr_layer_surface_v1::keyboard_interactivity::*;
        self.mapped && (self.current.keyboard == EXCLUSIVE || (on_demand && self.current.keyboard == ON_DEMAND))
    }
}

impl zwlr_layer_shell_v1::Handler for State {
    fn get_layer_surface(&mut self, cx: Cx, id: Obj, surface: u32, output: Option<u32>, layer: u32, _: String) -> Res {
        use zwlr_layer_shell_v1::error::{ALREADY_CONSTRUCTED, INVALID_LAYER, ROLE};
        let c = cx.client;
        let output = output.and_then(|o| self.client(c).output(o));
        let output = output.or_else(|| crate::output::focused(self)).unwrap_or_default();
        cx.error_if(layer > zwlr_layer_shell_v1::layer::OVERLAY, INVALID_LAYER, "invalid layer")?;
        if self.surfaces.get(&(c, surface)).is_some_and(|s| s.buffer.is_some() || s.pending.attached().is_some()) {
            return Err(cx.error(ALREADY_CONSTRUCTED, "surface has a buffer"));
        }
        self.client(c).set_data(id.id, surface);
        let layer = Layer {
            obj: id,
            output,
            current: LayerState { layer, ..Default::default() },
            rect: Rect::default(),
            configures: Configures::default(),
            committed: false,
            mapped: false,
        };
        surface::assign_role(self, (c, surface), Role::Layer(layer)).map_err(|m| cx.error(ROLE, m))?;
        self.layers.push((c, surface));
        Ok(())
    }
}

fn pending(st: &mut State, cx: Cx) -> Option<&mut LayerChanges> {
    st.surfaces.get_mut(&cx.surface()).filter(|s| matches!(s.role, Role::Layer(_))).map(|s| &mut s.pending.layer)
}

impl zwlr_layer_surface_v1::Handler for State {
    fn set_size(&mut self, cx: Cx, width: u32, height: u32) {
        if let Some(p) = pending(self, cx) {
            p.size = Some((width, height));
        }
    }

    fn set_anchor(&mut self, cx: Cx, anchor: u32) -> Res {
        cx.error_if(anchor > 15, zwlr_layer_surface_v1::error::INVALID_ANCHOR, "invalid anchor")?;
        if let Some(p) = pending(self, cx) {
            p.anchor = Some(anchor);
        }
        Ok(())
    }

    fn set_exclusive_zone(&mut self, cx: Cx, zone: i32) {
        if let Some(p) = pending(self, cx) {
            p.zone = Some(zone);
        }
    }

    fn set_margin(&mut self, cx: Cx, top: i32, right: i32, bottom: i32, left: i32) {
        if let Some(p) = pending(self, cx) {
            p.margin = Some([top, right, bottom, left]);
        }
    }

    fn set_keyboard_interactivity(&mut self, cx: Cx, keyboard_interactivity: u32) -> Res {
        use zwlr_layer_surface_v1::keyboard_interactivity::{EXCLUSIVE, ON_DEMAND};
        if keyboard_interactivity > ON_DEMAND || (cx.obj.version < 4 && keyboard_interactivity > EXCLUSIVE) {
            let code = zwlr_layer_surface_v1::error::INVALID_KEYBOARD_INTERACTIVITY;
            return Err(cx.error(code, "invalid keyboard interactivity"));
        }
        if let Some(p) = pending(self, cx) {
            p.keyboard = Some(keyboard_interactivity);
        }
        Ok(())
    }

    fn get_popup(&mut self, cx: Cx, popup: u32) {
        let c = cx.client;
        let Some(surface) = self.client(c).get_of(popup, Interface::XdgPopup).map(|o| o.data) else { return };
        if let Some(Role::Popup(p)) = self.surfaces.get_mut(&(c, surface)).map(|s| &mut s.role) {
            p.parent = Some(cx.data);
        }
    }

    fn ack_configure(&mut self, cx: Cx, serial: u32) -> Res {
        let Some(l) = layer_mut(self, cx.surface()) else { return Ok(()) };
        let invalid = zwlr_layer_surface_v1::error::INVALID_SURFACE_STATE;
        cx.error_if(!l.configures.ack(serial), invalid, "invalid configure serial")
    }

    fn destroy(&mut self, cx: Cx) {
        unmap(self, cx.surface());
    }

    fn set_layer(&mut self, cx: Cx, layer: u32) -> Res {
        if layer > zwlr_layer_shell_v1::layer::OVERLAY {
            return Err(cx.error(zwlr_layer_shell_v1::error::INVALID_LAYER, "invalid layer"));
        }
        if let Some(p) = pending(self, cx) {
            p.layer = Some(layer);
        }
        Ok(())
    }
}

pub fn check_commit(st: &State, key: Key, size: Option<(i32, i32)>) -> Res {
    use zwlr_layer_surface_v1::error::*;
    let Some(surface) = st.surfaces.get(&key) else { return Ok(()) };
    let Role::Layer(l) = &surface.role else { return Ok(()) };
    if size.is_some() && l.configures.acked().is_none() {
        return Err(Error::new(l.obj.id, INVALID_SURFACE_STATE, "buffer attached before the first ack"));
    }
    let s = surface::upcoming(surface).fold(l.current.clone(), |base, p| p.layer.over(base));
    let has = |edge| s.anchor & edge != 0;
    if (s.size.0 == 0 && !(has(anchor::LEFT) && has(anchor::RIGHT)))
        || (s.size.1 == 0 && !(has(anchor::TOP) && has(anchor::BOTTOM)))
    {
        return Err(Error::new(l.obj.id, INVALID_SIZE, "zero size requires anchoring to opposite edges"));
    }
    Ok(())
}

pub fn keyboard_focus(st: &State, on_demand: bool) -> Option<Key> {
    let wants = |k: &&Key| match layer(st, **k) {
        Some(l) => matches!(l.kind(), LayerKind::Top | LayerKind::Overlay) && l.wants_keyboard(on_demand),
        None => false,
    };
    st.layers.iter().filter(wants).max_by_key(|&&k| layer(st, k).map(Layer::kind)).copied()
}

fn layer(st: &State, key: Key) -> Option<&Layer> {
    match &st.surfaces.get(&key)?.role {
        Role::Layer(l) => Some(l),
        _ => None,
    }
}

fn layer_mut(st: &mut State, key: Key) -> Option<&mut Layer> {
    match &mut st.surfaces.get_mut(&key)?.role {
        Role::Layer(l) => Some(l),
        _ => None,
    }
}

pub fn committed(st: &mut State, key: Key, changes: LayerChanges) {
    let buffer = st.surfaces.get(&key).is_some_and(|s| s.buffer.is_some());
    let Some(l) = layer_mut(st, key) else { return };
    changes.apply(&mut l.current);
    let unmapped = l.mapped && !buffer;
    (l.mapped, l.committed) = (buffer, !unmapped);
    if unmapped {
        l.configures.unmap();
        crate::xdg::dismiss_children(st, key);
    }
    arrange(st);
}

pub fn unmap(st: &mut State, key: Key) {
    if layer(st, key).is_some() {
        crate::xdg::dismiss_children(st, key);
    }
    if let Some(s) = st.surfaces.get_mut(&key).filter(|s| matches!(s.role, Role::Layer(_))) {
        s.role = Role::None;
    }
    if st.layers.contains(&key) {
        st.layers.retain(|&k| k != key);
        arrange(st);
    }
}

fn close_orphans(st: &mut State) {
    let orphan = |l: &Layer| crate::output::find(st, l.output).is_none_or(|o| !o.in_layout());
    let gone: Vec<(Key, Obj)> =
        st.layers.iter().filter_map(|&k| layer(st, k).filter(|l| orphan(l)).map(|l| (k, l.obj))).collect();
    for (key, obj) in gone {
        st.send(key.0, |out| zwlr_layer_surface_v1::closed(out, obj));
        if let Some(s) = st.surfaces.get_mut(&key) {
            s.role = Role::None;
        }
        st.layers.retain(|&k| k != key);
    }
}

pub fn visible(st: &State, key: Key, kind: LayerKind) -> Option<Rect> {
    let surface = st.surfaces.get(&key)?;
    match &surface.role {
        Role::Layer(l) if l.kind() == kind && l.configures.last().is_some() && surface.buffer.is_some() => {
            crate::output::find(st, l.output).filter(|o| o.in_layout()).map(|_| l.rect)
        }
        _ => None,
    }
}

pub fn arrange(st: &mut State) {
    close_orphans(st);
    let mut configures = vec![];
    let mut changed = false;
    for index in 0..st.outputs.len() {
        let (id, full) = (st.outputs[index].id, st.outputs[index].rect);
        let mut usable = full;
        let on = |k: &Key| layer(st, *k).is_some_and(|l| l.committed && l.output == id);
        let (exclusive, rest): (Vec<Key>, Vec<Key>) =
            st.layers.iter().copied().filter(on).partition(|&k| layer(st, k).is_some_and(|l| l.current.zone > 0));
        for key in exclusive.into_iter().chain(rest) {
            let Some(l) = layer_mut(st, key) else { continue };
            let s = &l.current;
            l.rect = place(s, if s.zone < 0 { full } else { usable });
            if s.zone > 0 {
                usable = shrink(usable, s);
            }
            if l.configures.last() != Some((l.rect.w, l.rect.h)) {
                configures.push(key);
            }
        }
        changed |= std::mem::replace(&mut st.outputs[index].usable, usable) != usable;
    }
    for key in configures {
        let serial = st.next_serial();
        let Some(l) = layer_mut(st, key) else { continue };
        let (obj, (w, h)) = (l.obj, (l.rect.w, l.rect.h));
        l.configures.send(serial, (w, h));
        st.send(key.0, |out| zwlr_layer_surface_v1::configure(out, obj, serial, w as u32, h as u32));
    }
    if changed {
        crate::output::notify_wm(st);
    }
    st.render_dirty = true;
}

fn place(s: &LayerState, bounds: Rect) -> Rect {
    let [top, right, bottom, left] = s.margin;
    let has = |edge| s.anchor & edge != 0;
    let (x, w) = axis(bounds.x, bounds.w, s.size.0 as i32, (has(anchor::LEFT), has(anchor::RIGHT)), (left, right));
    let (y, h) = axis(bounds.y, bounds.h, s.size.1 as i32, (has(anchor::TOP), has(anchor::BOTTOM)), (top, bottom));
    Rect::new(x, y, w, h)
}

fn axis(start: i32, len: i32, size: i32, anchored: (bool, bool), (low, high): (i32, i32)) -> (i32, i32) {
    let size = if size == 0 && anchored == (true, true) { (len - low - high).max(0) } else { size };
    let position = match anchored {
        (true, false) => start + low,
        (false, true) => start + len - high - size,
        _ => start + low + (len - low - high - size) / 2,
    };
    (position, size)
}

fn shrink(r: Rect, s: &LayerState) -> Rect {
    let [top, right, bottom, left] = s.margin;
    let has = |edge| s.anchor & edge != 0;
    let (t, b, l, rt) = (has(anchor::TOP), has(anchor::BOTTOM), has(anchor::LEFT), has(anchor::RIGHT));
    let zone = s.zone;
    match (t, b, l, rt) {
        (true, false, ..) if l == rt => Rect { y: r.y + zone + top, h: r.h - zone - top, ..r },
        (false, true, ..) if l == rt => Rect { h: r.h - zone - bottom, ..r },
        (.., true, false) if t == b => Rect { x: r.x + zone + left, w: r.w - zone - left, ..r },
        (.., false, true) if t == b => Rect { w: r.w - zone - right, ..r },
        _ => r,
    }
}
