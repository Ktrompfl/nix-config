use crate::{
    ClientId, Key, State,
    client::Data,
    cursor::Cursor,
    scene::{self, Item},
    seat,
    selection::{self, Proto, source, source_mut},
    surface::{self, Role},
    xdg,
};
use std::{mem, os::fd::OwnedFd};
use wire::*;
use wl_data_device_manager::dnd_action::{ASK, COPY, MOVE};
use wm::Rect;

const ALL: u32 = COPY | MOVE | ASK;

pub struct Drag {
    source: Option<Key>,
    origin: ClientId,
    icon: Option<(Key, (i32, i32))>,
    target: Option<Key>,
    offers: Vec<Key>,
}

pub struct Offer {
    actions: u32,
    preferred: u32,
    action: u32,
    accepted: bool,
    dropped: Option<u32>,
}

pub fn active(st: &State) -> bool {
    st.drag.is_some()
}

pub fn start(st: &mut State, cx: Cx, source: Option<u32>, icon: Option<u32>) -> Res {
    let (c, obj) = (cx.client, cx.obj);
    let source = source.map(|id| (c, id));
    if let Some(s) = source.and_then(|k| source_mut(st, k)) {
        if s.used {
            return Err(Error::new(obj.id, wl_data_device::error::USED_SOURCE, "source already used"));
        }
        s.used = true;
    }
    if active(st) || !seat::holds_button(st, c) {
        source.into_iter().for_each(|s| cancel(st, s));
        return Ok(());
    }
    if let Some(icon) = icon.map(|id| (c, id))
        && !matches!(st.surfaces.get(&icon).map(|s| &s.role), Some(Role::DndIcon))
    {
        surface::assign_role(st, icon, Role::DndIcon)
            .map_err(|m| Error::new(obj.id, wl_data_device::error::ROLE, m))?;
    }
    let icon = icon.map(|id| ((c, id), (0, 0)));
    st.drag = Some(Drag { source, origin: c, icon, target: None, offers: vec![] });
    seat::leave_focus(st);
    st.cursor = Cursor::Named("grabbing");
    if let Some((x, y)) = st.seat.pointer() {
        motion(st, x, y);
    }
    Ok(())
}

pub fn motion(st: &mut State, x: f64, y: f64) {
    st.render_dirty = true;
    let Some(drag) = &st.drag else { return };
    let (origin, anonymous, target) = (drag.origin, drag.source.is_none(), drag.target);
    let attached = drag.source.and_then(|s| source(st, s)?.toplevel_drag?.1);
    let hit = scene::surface_at(st, x, y).filter(|h| (!anonymous || h.0.0 == origin) && Some(h.0) != attached);
    if hit.map(|h| h.0) != target {
        leave(st);
        if let Some((key, sx, sy)) = hit {
            enter(st, key, sx, sy);
        }
    } else if let Some((key, sx, sy)) = hit {
        let time = st.time();
        devices(st, key.0, |out, d| wl_data_device::motion(out, d, time, sx.into(), sy.into()));
    }
}

fn enter(st: &mut State, key: Key, sx: f64, sy: f64) {
    let Some(source) = st.drag.as_ref().map(|d| d.source) else { return };
    let mimes = source.and_then(|s| self::source(st, s)).map(|s| s.mimes.clone()).unwrap_or_default();
    let source_actions = source.map_or(0, |s| actions_of(st, s));
    let serial = st.next_serial();
    let Some(client) = st.clients.get_mut(&key.0) else { return };
    let mut offers = vec![];
    for device in client.of(Interface::WlDataDevice).collect::<Vec<_>>() {
        let offer = source.map(|source| {
            let (actions, accepted) = if device.version >= 3 { (0, false) } else { (COPY, true) };
            let dnd = Offer { actions, preferred: 0, action: 0, accepted, dropped: None };
            let offer = Data::Offer(selection::Offer { source, dnd: Some(dnd) });
            let offer = client.add_server(Interface::WlDataOffer, device.version, offer);
            wl_data_device::data_offer(&mut client.out, device, offer.id);
            mimes.iter().for_each(|m| wl_data_offer::offer(&mut client.out, offer, m));
            wl_data_offer::source_actions(&mut client.out, offer, source_actions);
            offers.push((key.0, offer.id));
            offer.id
        });
        wl_data_device::enter(&mut client.out, device, serial, key.1, sx.into(), sy.into(), offer);
    }
    for &k in &offers {
        negotiate(st, k);
    }
    if let Some(d) = st.drag.as_mut() {
        d.offers.extend(offers);
    }
    st.drag.iter_mut().for_each(|d| d.target = Some(key));
}

fn leave(st: &mut State) {
    let Some(drag) = st.drag.as_mut() else { return };
    let Some(target) = drag.target.take() else { return };
    let (offers, source) = (mem::take(&mut drag.offers), drag.source);
    forget_offers(st, &offers);
    devices(st, target.0, wl_data_device::leave);
    if let Some(source) = source {
        to_source(st, source, |out, s| {
            wl_data_source::target(out, s, None);
            wl_data_source::action(out, s, 0);
        });
    }
}

pub fn release(st: &mut State) {
    let Some(drag) = &st.drag else { return };
    let ready = |&k: &Key| matches!(st.data(k), Some(Data::Offer(o)) if o.dnd.as_ref().is_some_and(|o| o.accepted && o.action != 0));
    let offer = drag.offers.iter().copied().find(ready);
    let rest: Vec<Key> = drag.offers.iter().copied().filter(|&k| Some(k) != offer).collect();
    let source = drag.source;
    let Some(target) = drag.target.filter(|_| source.is_none() || offer.is_some()) else { return abort(st) };
    if let (Some(source), Some(offer)) = (source, offer) {
        forget_offers(st, &rest);
        dnd_offer_mut(st, offer).into_iter().for_each(|(o, _)| o.dropped = Some(o.action));
        to_source(st, source, wl_data_source::dnd_drop_performed);
    }
    devices(st, target.0, |out, d| {
        wl_data_device::drop(out, d);
        wl_data_device::leave(out, d);
    });
    end(st);
}

fn end(st: &mut State) {
    st.drag = None;
    st.cursor = Cursor::Named("default");
    st.render_dirty = true;
    seat::rehover(st);
}

pub fn abort(st: &mut State) {
    let source = st.drag.as_ref().and_then(|d| d.source);
    leave(st);
    source.into_iter().for_each(|s| cancel(st, s));
    end(st);
}

fn cancel(st: &mut State, source: Key) {
    if selection::source_object(st, source).is_some_and(|(_, obj)| obj.version >= 3) {
        selection::cancel(st, source);
    }
}

fn dnd_offer_mut(st: &mut State, key: Key) -> Option<(&mut Offer, Key)> {
    let o = selection::offer_mut(st, key)?;
    Some((o.dnd.as_mut()?, o.source))
}

impl wl_data_offer::Handler for State {
    fn accept(&mut self, cx: Cx, _: u32, mime_type: Option<String>) {
        let Some((o, source)) = dnd_offer_mut(self, cx.key()) else { return };
        o.accepted = mime_type.is_some() || cx.obj.version < 3;
        to_source(self, source, |out, s| wl_data_source::target(out, s, mime_type.as_deref()));
    }

    fn receive(&mut self, cx: Cx, mime_type: String, fd: OwnedFd) {
        selection::receive(self, cx.key(), &mime_type, fd);
    }

    fn finish(&mut self, cx: Cx) -> Res {
        let Some((o, source)) = dnd_offer_mut(self, cx.key()) else {
            return not_dnd(self, cx, wl_data_offer::error::INVALID_FINISH);
        };
        if o.dropped.is_none() || o.action == 0 || !o.accepted {
            return Err(cx.error(wl_data_offer::error::INVALID_FINISH, "finish without a drop"));
        }
        let (dropped, action) = (o.dropped, o.action);
        if let Some(o) = selection::offer_mut(self, cx.key()) {
            o.dnd = None;
        }
        if dropped == Some(ASK) {
            to_source(self, source, |out, s| wl_data_source::action(out, s, action));
        }
        to_source(self, source, wl_data_source::dnd_finished);
        Ok(())
    }

    fn set_actions(&mut self, cx: Cx, dnd_actions: u32, preferred_action: u32) -> Res {
        use wl_data_offer::error::{INVALID_ACTION, INVALID_ACTION_MASK, INVALID_OFFER};
        cx.error_if(dnd_actions & !ALL != 0, INVALID_ACTION_MASK, "unknown actions")?;
        if preferred_action & !ALL != 0 || preferred_action.count_ones() > 1 {
            return Err(cx.error(INVALID_ACTION, "invalid preferred action"));
        }
        let Some((o, source)) = dnd_offer_mut(self, cx.key()) else { return not_dnd(self, cx, INVALID_OFFER) };
        let dropped = o.dropped;
        match dropped {
            Some(ASK) if preferred_action & !actions_of(self, source) != 0 => {
                return Err(cx.error(INVALID_ACTION, "action not offered"));
            }
            Some(ASK) | None => {}
            Some(_) => return Ok(()),
        }
        if let Some((o, _)) = dnd_offer_mut(self, cx.key()) {
            (o.actions, o.preferred) = (dnd_actions, preferred_action);
        }
        negotiate(self, cx.key());
        Ok(())
    }
}

pub fn offer_gone(st: &mut State, obj: Obj, offer: selection::Offer) {
    if offer.dnd.is_some_and(|o| o.dropped.is_some()) {
        match obj.version {
            3.. => cancel(st, offer.source),
            _ => to_source(st, offer.source, wl_data_source::dnd_finished),
        }
    }
}

pub fn source_actions(st: &mut State, cx: Cx, actions: u32) -> Res {
    cx.error_if(actions & !ALL != 0, wl_data_source::error::INVALID_ACTION_MASK, "unknown actions")?;
    let Some(s) = source_mut(st, cx.key()) else { return Ok(()) };
    if s.used || s.actions.is_some() {
        let message = "set_actions after use or twice";
        return Err(cx.error(wl_data_source::error::INVALID_SOURCE, message));
    }
    s.actions = Some(actions);
    Ok(())
}

fn not_dnd(st: &State, cx: Cx, code: u32) -> Res {
    match st.data(cx.key()) {
        Some(Data::Offer(_)) => Err(cx.error(code, "not a drag-and-drop offer")),
        _ => Ok(()),
    }
}

fn negotiate(st: &mut State, key: Key) {
    let Some((o, source)) = dnd_offer_mut(st, key) else { return };
    let (actions, preferred) = (o.actions, o.preferred);
    let both = actions_of(st, source) & actions;
    let action = match both & preferred {
        0 => [COPY, MOVE, ASK].into_iter().find(|a| both & a != 0).unwrap_or(0),
        preferred => preferred,
    };
    let Some((o, _)) = dnd_offer_mut(st, key).filter(|o| o.0.action != action) else { return };
    o.action = action;
    if o.dropped.is_some() {
        return;
    }
    let offer = st.clients.get(&key.0).and_then(|c| c.get(key.1)).map(|o| o.obj);
    if let Some(offer) = offer {
        st.send(key.0, |out| wl_data_offer::action(out, offer, action));
    }
    to_source(st, source, |out, s| wl_data_source::action(out, s, action));
}

fn actions_of(st: &State, source: Key) -> u32 {
    self::source(st, source).and_then(|s| s.actions).unwrap_or(COPY)
}

fn to_source(st: &mut State, source: Key, f: impl FnOnce(&mut Out, Obj)) {
    if let Some((Proto::Data, obj)) = selection::source_object(st, source) {
        st.send(source.0, |out| f(out, obj));
    }
}

fn devices(st: &mut State, c: ClientId, f: impl Fn(&mut Out, Obj)) {
    if let Some(client) = st.clients.get_mut(&c) {
        client.each(Interface::WlDataDevice, f);
    }
}

fn forget_offers(st: &mut State, offers: &[Key]) {
    for &offer in offers {
        if let Some(d) = st.data_mut(offer).filter(|d| matches!(d, Data::Offer(_))) {
            *d = Data::None;
        }
    }
}

pub fn icon(st: &State) -> Option<Item> {
    let (key, (dx, dy)) = st.drag.as_ref()?.icon?;
    let (x, y) = st.seat.pointer()?;
    let (w, h) = st.surfaces.get(&key)?.size();
    let origin = (x as i32 + dx, y as i32 + dy);
    Some(Item::Surface { key, origin, clip: Rect::new(origin.0, origin.1, w, h) })
}

pub fn offset(st: &mut State, key: Key, (dx, dy): (i32, i32)) {
    if let Some((icon, offset)) = st.drag.as_mut().and_then(|d| d.icon.as_mut())
        && *icon == key
    {
        *offset = (offset.0 + dx, offset.1 + dy);
    }
}

impl xdg_toplevel_drag_manager_v1::Handler for State {
    fn get_xdg_toplevel_drag(&mut self, cx: Cx, id: Obj, data_source: u32) -> Res {
        self.set((cx.client, id.id), Data::ToplevelDrag(data_source));
        let Some(s) = source_mut(self, (cx.client, data_source)) else { return Ok(()) };
        if s.toplevel_drag.is_some() || s.used {
            return Err(cx.error(xdg_toplevel_drag_manager_v1::error::INVALID_SOURCE, "source already used"));
        }
        s.toplevel_drag = Some((id.id, None));
        Ok(())
    }
}

fn toplevel_drag(st: &mut State, cx: Cx) -> Option<&mut Option<Key>> {
    let &mut Data::ToplevelDrag(source) = st.data_mut(cx.key())? else { return None };
    source_mut(st, (cx.client, source))?.toplevel_drag.as_mut().filter(|d| d.0 == cx.obj.id).map(|d| &mut d.1)
}

impl xdg_toplevel_drag_v1::Handler for State {
    fn destroy(&mut self, cx: Cx) -> Res {
        let Some(&mut Data::ToplevelDrag(source)) = self.data_mut(cx.key()) else { return Ok(()) };
        if self.drag.as_ref().is_some_and(|d| d.source == Some((cx.client, source))) {
            return Err(cx.error(xdg_toplevel_drag_v1::error::ONGOING_DRAG, "drag still ongoing"));
        }
        if toplevel_drag(self, cx).is_some()
            && let Some(s) = source_mut(self, (cx.client, source))
        {
            s.toplevel_drag = None;
        }
        Ok(())
    }

    fn attach(&mut self, cx: Cx, toplevel: u32, _: i32, _: i32) -> Res {
        let window = xdg::toplevel_object(self, cx.client, toplevel);
        let Some(attached) = toplevel_drag(self, cx) else { return Ok(()) };
        cx.error_if(attached.is_some(), xdg_toplevel_drag_v1::error::TOPLEVEL_ATTACHED, "toplevel attached")?;
        *attached = window;
        Ok(())
    }
}

pub fn toplevel_gone(st: &mut State, window: Key) {
    let Some(client) = st.clients.get_mut(&window.0) else { return };
    for o in client.objects_mut() {
        if let Data::Source(selection::Source { toplevel_drag: Some((_, w)), .. }) = &mut o.state
            && *w == Some(window)
        {
            *w = None;
        }
    }
}

pub fn source_gone(st: &mut State, source: Key) {
    if st.drag.as_ref().is_some_and(|d| d.source == Some(source)) {
        drop_source(st);
    }
}

fn drop_source(st: &mut State) {
    st.drag.iter_mut().for_each(|d| d.source = None);
    leave(st);
    end(st);
}

pub fn surface_gone(st: &mut State, key: Key) {
    let Some(drag) = st.drag.as_mut() else { return };
    if drag.icon.is_some_and(|i| i.0 == key) {
        drag.icon = None;
    }
    if drag.target == Some(key) {
        leave(st);
    }
}

pub fn client_gone(st: &mut State, c: ClientId) {
    if st.drag.as_ref().is_some_and(|d| d.origin == c) {
        drop_source(st);
    }
}
