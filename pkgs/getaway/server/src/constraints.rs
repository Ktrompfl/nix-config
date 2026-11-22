use crate::{Key, State, base, scene, seat, surface::Changes};
use wire::*;
use wm::{Rect, Region};

pub struct Constraint {
    obj: Obj,
    lock: bool,
    persistent: bool,
    dead: bool,
    region: Option<Region>,
}

impl zwp_pointer_constraints_v1::Handler for State {
    fn lock_pointer(&mut self, cx: Cx, id: Obj, surface: u32, _: u32, region: Option<u32>, lifetime: u32) -> Res {
        constrain(self, cx, id, surface, region, lifetime, true)
    }

    fn confine_pointer(&mut self, cx: Cx, id: Obj, surface: u32, _: u32, region: Option<u32>, lifetime: u32) -> Res {
        constrain(self, cx, id, surface, region, lifetime, false)
    }
}

fn constrain(st: &mut State, cx: Cx, id: Obj, surface: u32, region: Option<u32>, lifetime: u32, lock: bool) -> Res {
    let c = cx.client;
    let region = base::region(st, c, region);
    let Some(s) = st.surfaces.get_mut(&(c, surface)) else { return Ok(()) };
    if s.constraint.as_ref().is_some_and(|k| !k.dead) {
        return Err(cx.error(zwp_pointer_constraints_v1::error::ALREADY_CONSTRAINED, "surface is already constrained"));
    }
    let persistent = lifetime == zwp_pointer_constraints_v1::lifetime::PERSISTENT;
    s.constraint = Some(Constraint { obj: id, lock, persistent, dead: false, region });
    st.client(c).set_data(id.id, surface);
    update(st);
    Ok(())
}

impl zwp_locked_pointer_v1::Handler for State {
    fn set_cursor_position_hint(&mut self, cx: Cx, surface_x: Fixed, surface_y: Fixed) {
        if let Some(s) = pending(self, cx) {
            s.pointer_hint = Some(Some((cx.obj.id, (surface_x.into(), surface_y.into()))));
        }
    }

    fn set_region(&mut self, cx: Cx, region: Option<u32>) {
        set_region(self, cx, region);
    }
}

impl zwp_confined_pointer_v1::Handler for State {
    fn set_region(&mut self, cx: Cx, region: Option<u32>) {
        set_region(self, cx, region);
    }
}

fn set_region(st: &mut State, cx: Cx, region: Option<u32>) {
    let region = base::region(st, cx.client, region);
    if let Some(s) = pending(st, cx) {
        s.pointer_region = Some(Some((cx.obj.id, region)));
    }
}

fn constraint(st: &State, surface: Key) -> Option<&Constraint> {
    st.surfaces.get(&surface)?.constraint.as_ref()
}

fn pending(st: &mut State, cx: Cx) -> Option<&mut Changes> {
    let s = st.surfaces.get_mut(&cx.surface()).filter(|s| s.constraint.as_ref().is_some_and(|k| k.obj == cx.obj))?;
    Some(&mut s.pending.state)
}

fn hint(st: &State, surface: Key) -> Option<(f64, f64)> {
    let s = st.surfaces.get(&surface)?;
    let id = s.constraint.as_ref()?.obj.id;
    s.pointer_hint.filter(|h| h.0 == id).map(|h| h.1)
}

fn region(st: &State, surface: Key) -> Option<&Region> {
    let s = st.surfaces.get(&surface)?;
    let k = s.constraint.as_ref()?;
    match &s.pointer_region {
        Some((id, region)) if *id == k.obj.id => region.as_ref(),
        _ => k.region.as_ref(),
    }
}

pub fn confine(st: &State, surface: Key, (x, y): (f64, f64)) -> (f64, f64) {
    let (Some(visible), Some(full)) = (scene::visible_rect(st, surface), scene::surface_rect(st, surface)) else {
        return (x, y);
    };
    let region = region(st, surface).map(Region::bounds);
    let r = region.map_or(visible, |b| visible.intersect(Rect::new(full.x + b.x, full.y + b.y, b.w, b.h)));
    if r.is_empty() {
        return (x, y);
    }
    (x.clamp(f64::from(r.x), f64::from(r.right() - 1)), y.clamp(f64::from(r.y), f64::from(r.bottom() - 1)))
}

impl zwp_relative_pointer_manager_v1::Handler for State {
    fn get_relative_pointer(&mut self, _: Cx, _: Obj, _: u32) {}
}

impl zwp_relative_pointer_v1::Handler for State {}

pub fn gone(st: &mut State, surface: Key, obj: Obj) {
    if constraint(st, surface).is_some_and(|k| k.obj == obj) {
        remove(st, surface);
    }
}

pub fn remove(st: &mut State, surface: Key) {
    let hint = hint(st, surface);
    if let Some(s) = st.surfaces.get_mut(&surface) {
        s.constraint = None;
    }
    let released = st.seat.constrained.take_if(|&mut k| k == surface).is_some();
    if released {
        st.backend.constrain(None);
    }
    update(st);
    if released {
        restore(st, surface, hint);
    }
}

pub fn update(st: &mut State) {
    let (pointer, keyboard) = (seat::pointer_target(st), st.seat.keyboard_focus());
    let (x, y) = st.seat.local();
    let active = st.seat.constrained;
    let wanted = pointer.filter(|&k| {
        let inside = active == Some(k) || region(st, k).is_none_or(|r| r.contains(x, y));
        constraint(st, k).is_some_and(|c| !c.dead) && Some(scene::root(st, k)) == keyboard && inside
    });
    if wanted == active {
        return;
    }
    let released = active.map(|k| (k, hint(st, k)));
    if let Some(k) = active {
        notify(st, k, false);
    }
    if let Some(k) = wanted {
        notify(st, k, true);
    }
    st.seat.constrained = wanted;
    let lock = wanted.and_then(|k| constraint(st, k)).map(|k| k.lock);
    st.backend.constrain(lock);
    if let Some((surface, hint)) = released {
        restore(st, surface, hint);
    }
}

fn restore(st: &mut State, surface: Key, hint: Option<(f64, f64)>) {
    let Some((x, y)) = hint.filter(|_| seat::pointer_target(st) == Some(surface)) else { return };
    let Some(r) = scene::surface_rect(st, surface) else { return };
    let (x, y) = (f64::from(r.x) + x, f64::from(r.y) + y);
    if r.contains(x, y) {
        seat::warp_to(st, x, y);
    }
}

fn notify(st: &mut State, surface: Key, on: bool) {
    let Some(k) = st.surfaces.get_mut(&surface).and_then(|s| s.constraint.as_mut()) else { return };
    k.dead |= !on && !k.persistent;
    let (obj, lock) = (k.obj, k.lock);
    log::debug!("{} {surface:?} {}", if lock { "lock" } else { "confinement" }, if on { "on" } else { "off" });
    st.send(surface.0, |out| match (lock, on) {
        (true, true) => zwp_locked_pointer_v1::locked(out, obj),
        (true, false) => zwp_locked_pointer_v1::unlocked(out, obj),
        (false, true) => zwp_confined_pointer_v1::confined(out, obj),
        (false, false) => zwp_confined_pointer_v1::unconfined(out, obj),
    });
}

pub fn yield_to_warp(st: &mut State, x: f64, y: f64) {
    let Some((k, lock)) = active(st) else { return };
    if !lock && confine(st, k, (x, y)) == (x, y) {
        return;
    }
    notify(st, k, false);
    st.seat.constrained = None;
    st.backend.constrain(None);
}

pub fn active(st: &State) -> Option<(Key, bool)> {
    st.seat.constrained.and_then(|k| Some((k, constraint(st, k)?.lock)))
}

pub fn relative_motion(st: &mut State, delta: [f64; 4], usec: u64) {
    let Some(client) = seat::pointer_target(st).and_then(|k| st.clients.get_mut(&k.0)) else { return };
    let (hi, lo) = ((usec >> 32) as u32, usec as u32);
    let [dx, dy, dx_raw, dy_raw] = delta.map(Fixed::from);
    client.each(Interface::ZwpRelativePointerV1, |out, relative| {
        zwp_relative_pointer_v1::relative_motion(out, relative, hi, lo, dx, dy, dx_raw, dy_raw)
    });
    client.each(Interface::WlPointer, wl_pointer::frame);
}
