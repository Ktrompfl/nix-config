use crate::{ClientId, Key, State, caps, client::Data, dnd};
use std::{
    mem,
    os::fd::{AsFd, BorrowedFd, OwnedFd},
};
use wire::*;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Sel {
    Clipboard,
    Primary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Proto {
    Data,
    Primary,
    Ext,
    Wlr,
}

impl Proto {
    fn serves(self, sel: Sel) -> bool {
        match self {
            Proto::Data => sel == Sel::Clipboard,
            Proto::Primary => sel == Sel::Primary,
            Proto::Ext | Proto::Wlr => true,
        }
    }

    fn control(self) -> bool {
        matches!(self, Proto::Ext | Proto::Wlr)
    }
}

const SELECTIONS: [Sel; 2] = [Sel::Clipboard, Sel::Primary];

pub struct Source {
    pub proto: Proto,
    pub mimes: Vec<String>,
    pub actions: Option<u32>,
    pub used: bool,
    pub toplevel_drag: Option<(u32, Option<Key>)>,
}

pub struct Offer {
    pub source: Key,
    pub dnd: Option<dnd::Offer>,
}

pub fn source(st: &State, key: Key) -> Option<&Source> {
    match st.data(key)? {
        Data::Source(s) => Some(s),
        _ => None,
    }
}

pub fn source_mut(st: &mut State, key: Key) -> Option<&mut Source> {
    match st.data_mut(key)? {
        Data::Source(s) => Some(s),
        _ => None,
    }
}

pub fn offer_mut(st: &mut State, key: Key) -> Option<&mut Offer> {
    match st.data_mut(key)? {
        Data::Offer(o) => Some(o),
        _ => None,
    }
}

fn proto(interface: Interface) -> Option<Proto> {
    match interface {
        Interface::WlDataDevice => Some(Proto::Data),
        Interface::ZwpPrimarySelectionDeviceV1 => Some(Proto::Primary),
        Interface::ExtDataControlDeviceV1 => Some(Proto::Ext),
        Interface::ZwlrDataControlDeviceV1 => Some(Proto::Wlr),
        _ => None,
    }
}

fn devices(st: &State, c: Option<ClientId>) -> impl Iterator<Item = (ClientId, Proto, Obj)> + '_ {
    let clients = st.clients.iter().filter(move |e| c.is_none_or(|c| c == *e.0));
    clients.flat_map(|(&c, client)| client.objects().filter_map(move |o| Some((c, proto(o.interface)?, o.obj))))
}

impl wl_data_device_manager::Handler for State {
    fn create_data_source(&mut self, cx: Cx, id: Obj) {
        create_source(self, cx, id, Proto::Data);
    }

    fn get_data_device(&mut self, cx: Cx, id: Obj, _: u32) {
        create_device(self, cx, id, Proto::Data);
    }
}

impl wl_data_source::Handler for State {
    fn offer(&mut self, cx: Cx, mime_type: String) -> Res {
        add_mime(self, cx.key(), mime_type)
    }

    fn set_actions(&mut self, cx: Cx, dnd_actions: u32) -> Res {
        dnd::source_actions(self, cx, dnd_actions)
    }
}

impl wl_data_device::Handler for State {
    fn start_drag(&mut self, cx: Cx, source: Option<u32>, _: u32, icon: Option<u32>, _: u32) -> Res {
        dnd::start(self, cx, source, icon)
    }

    fn set_selection(&mut self, cx: Cx, source: Option<u32>, _: u32) -> Res {
        set(self, cx, Proto::Data, Sel::Clipboard, source)
    }
}

impl zwp_primary_selection_device_manager_v1::Handler for State {
    fn create_source(&mut self, cx: Cx, id: Obj) {
        create_source(self, cx, id, Proto::Primary);
    }

    fn get_device(&mut self, cx: Cx, id: Obj, _: u32) {
        create_device(self, cx, id, Proto::Primary);
    }
}

impl zwp_primary_selection_device_v1::Handler for State {
    fn set_selection(&mut self, cx: Cx, source: Option<u32>, _: u32) -> Res {
        set(self, cx, Proto::Primary, Sel::Primary, source)
    }
}

impl zwp_primary_selection_source_v1::Handler for State {
    fn offer(&mut self, cx: Cx, mime_type: String) -> Res {
        add_mime(self, cx.key(), mime_type)
    }
}

impl zwp_primary_selection_offer_v1::Handler for State {
    fn receive(&mut self, cx: Cx, mime_type: String, fd: OwnedFd) {
        receive(self, cx.key(), &mime_type, fd);
    }
}

impl ext_data_control_manager_v1::Handler for State {
    fn create_data_source(&mut self, cx: Cx, id: Obj) {
        create_source(self, cx, id, Proto::Ext);
    }

    fn get_data_device(&mut self, cx: Cx, id: Obj, _: u32) {
        create_device(self, cx, id, Proto::Ext);
    }
}

impl ext_data_control_device_v1::Handler for State {
    fn set_selection(&mut self, cx: Cx, source: Option<u32>) -> Res {
        set(self, cx, Proto::Ext, Sel::Clipboard, source)
    }

    fn set_primary_selection(&mut self, cx: Cx, source: Option<u32>) -> Res {
        set(self, cx, Proto::Ext, Sel::Primary, source)
    }
}

impl ext_data_control_source_v1::Handler for State {
    fn offer(&mut self, cx: Cx, mime_type: String) -> Res {
        add_mime(self, cx.key(), mime_type)
    }
}

impl ext_data_control_offer_v1::Handler for State {
    fn receive(&mut self, cx: Cx, mime_type: String, fd: OwnedFd) {
        receive(self, cx.key(), &mime_type, fd);
    }
}

impl zwlr_data_control_manager_v1::Handler for State {
    fn create_data_source(&mut self, cx: Cx, id: Obj) {
        create_source(self, cx, id, Proto::Wlr);
    }

    fn get_data_device(&mut self, cx: Cx, id: Obj, _: u32) {
        create_device(self, cx, id, Proto::Wlr);
    }
}

impl zwlr_data_control_device_v1::Handler for State {
    fn set_selection(&mut self, cx: Cx, source: Option<u32>) -> Res {
        set(self, cx, Proto::Wlr, Sel::Clipboard, source)
    }

    fn set_primary_selection(&mut self, cx: Cx, source: Option<u32>) -> Res {
        set(self, cx, Proto::Wlr, Sel::Primary, source)
    }
}

impl zwlr_data_control_source_v1::Handler for State {
    fn offer(&mut self, cx: Cx, mime_type: String) -> Res {
        add_mime(self, cx.key(), mime_type)
    }
}

impl zwlr_data_control_offer_v1::Handler for State {
    fn receive(&mut self, cx: Cx, mime_type: String, fd: OwnedFd) {
        receive(self, cx.key(), &mime_type, fd);
    }
}

fn create_source(st: &mut State, cx: Cx, id: Obj, proto: Proto) {
    let source = Source { proto, mimes: vec![], actions: None, used: false, toplevel_drag: None };
    st.set((cx.client, id.id), Data::Source(source));
}

fn create_device(st: &mut State, cx: Cx, device: Obj, proto: Proto) {
    let c = cx.client;
    if proto.control() || st.seat.keyboard_client() == Some(c) {
        for sel in SELECTIONS.into_iter().filter(|&s| proto.serves(s)) {
            offer(st, c, proto, device, sel);
        }
    }
}

fn add_mime(st: &mut State, source: Key, mime: String) -> Res {
    let Some(s) = source_mut(st, source) else { return Ok(()) };
    if s.used && s.proto.control() {
        let code = match s.proto {
            Proto::Ext => ext_data_control_source_v1::error::INVALID_OFFER,
            _ => zwlr_data_control_source_v1::error::INVALID_OFFER,
        };
        return Err(Error::new(source.1, code, "offer after set_selection"));
    }
    s.mimes.push(mime);
    Ok(())
}

fn set(st: &mut State, cx: Cx, proto: Proto, sel: Sel, source: Option<u32>) -> Res {
    let (c, device) = (cx.client, cx.obj);
    let new = source.map(|id| (c, id)).filter(|&k| self::source(st, k).is_some_and(|s| s.proto == proto));
    if let Some((id, s)) = new.and_then(|k| Some((k.1, source_mut(st, k)?))) {
        if s.used {
            return used_source(proto, device);
        }
        if s.actions.is_some() {
            return Err(Error::new(id, wl_data_source::error::INVALID_SOURCE, "drag-and-drop source"));
        }
        s.used = true;
    }
    if !proto.control() && st.seat.keyboard_client() != Some(c) {
        new.into_iter().for_each(|k| cancel(st, k));
        return Ok(());
    }
    let old = mem::replace(&mut st.selections[sel as usize], new);
    if let Some(old) = old.filter(|&old| Some(old) != new) {
        cancel(st, old);
    }
    announce(st, sel);
    Ok(())
}

fn used_source(proto: Proto, device: Obj) -> Res {
    let code = match proto {
        Proto::Data => wl_data_device::error::USED_SOURCE,
        Proto::Ext => ext_data_control_device_v1::error::USED_SOURCE,
        Proto::Wlr if device.version >= 2 => zwlr_data_control_device_v1::error::USED_SOURCE,
        _ => return Ok(()),
    };
    Err(Error::new(device.id, code, "source already used"))
}

fn announce(st: &mut State, sel: Sel) {
    let focus = st.seat.keyboard_client();
    let receives = |c: ClientId, proto: Proto| proto.serves(sel) && (proto.control() || Some(c) == focus);
    let targets: Vec<_> = devices(st, None).filter(|d| receives(d.0, d.1)).collect();
    for (c, proto, device) in targets {
        offer(st, c, proto, device, sel);
    }
}

fn offer(st: &mut State, c: ClientId, proto: Proto, device: Obj, sel: Sel) {
    let source = st.selections[sel as usize];
    let mimes = source.and_then(|k| self::source(st, k)).map(|s| s.mimes.clone()).unwrap_or_default();
    let Some(client) = st.clients.get_mut(&c) else { return };
    macro_rules! send {
        ($device:ident, $offer:ident, $interface:ident, $selection:ident) => {{
            let offer = source.map(|source| {
                let offer = Data::Offer(Offer { source, dnd: None });
                client.add_server(Interface::$interface, device.version, offer)
            });
            if let Some(offer) = offer {
                $device::data_offer(&mut client.out, device, offer.id);
                mimes.iter().for_each(|m| $offer::offer(&mut client.out, offer, m));
            }
            $device::$selection(&mut client.out, device, offer.map(|o| o.id));
        }};
    }
    match (proto, sel) {
        (Proto::Data, _) => send!(wl_data_device, wl_data_offer, WlDataOffer, selection),
        (Proto::Primary, _) => send!(
            zwp_primary_selection_device_v1,
            zwp_primary_selection_offer_v1,
            ZwpPrimarySelectionOfferV1,
            selection
        ),
        (Proto::Ext, Sel::Clipboard) => {
            send!(ext_data_control_device_v1, ext_data_control_offer_v1, ExtDataControlOfferV1, selection)
        }
        (Proto::Ext, Sel::Primary) => {
            send!(ext_data_control_device_v1, ext_data_control_offer_v1, ExtDataControlOfferV1, primary_selection)
        }
        (Proto::Wlr, Sel::Clipboard) => {
            send!(zwlr_data_control_device_v1, zwlr_data_control_offer_v1, ZwlrDataControlOfferV1, selection)
        }
        (Proto::Wlr, Sel::Primary) => {
            send!(zwlr_data_control_device_v1, zwlr_data_control_offer_v1, ZwlrDataControlOfferV1, primary_selection)
        }
    }
}

pub fn source_object(st: &State, source: Key) -> Option<(Proto, Obj)> {
    let proto = self::source(st, source)?.proto;
    Some((proto, st.clients.get(&source.0)?.get(source.1)?.obj))
}

pub fn receive(st: &mut State, offer: Key, mime: &str, fd: OwnedFd) {
    let Some(source) = offer_mut(st, offer).map(|o| o.source) else { return };
    let Some((proto, obj)) = source_object(st, source) else { return };
    st.send(source.0, |out| request_data(out, proto, obj, mime, fd.as_fd()));
}

fn request_data(out: &mut Out, proto: Proto, source: Obj, mime: &str, fd: BorrowedFd) {
    match proto {
        Proto::Data => wl_data_source::send(out, source, mime, fd),
        Proto::Primary => zwp_primary_selection_source_v1::send(out, source, mime, fd),
        Proto::Ext => ext_data_control_source_v1::send(out, source, mime, fd),
        Proto::Wlr => zwlr_data_control_source_v1::send(out, source, mime, fd),
    }
}

pub fn cancel(st: &mut State, source: Key) {
    let Some((proto, obj)) = source_object(st, source) else { return };
    st.send(source.0, |out| match proto {
        Proto::Data => wl_data_source::cancelled(out, obj),
        Proto::Primary => zwp_primary_selection_source_v1::cancelled(out, obj),
        Proto::Ext => ext_data_control_source_v1::cancelled(out, obj),
        Proto::Wlr => zwlr_data_control_source_v1::cancelled(out, obj),
    });
}

pub fn source_gone(st: &mut State, source: Key) {
    st.each_object(caps::NONE, |_, _, o| {
        if matches!(&o.state, Data::Offer(offer) if offer.source == source) {
            o.state = Data::None;
        }
    });
    for sel in SELECTIONS {
        if st.selections[sel as usize] == Some(source) {
            st.selections[sel as usize] = None;
            announce(st, sel);
        }
    }
    dnd::source_gone(st, source);
}

pub fn focus_changed(st: &mut State) {
    let Some(focus) = st.seat.keyboard_client() else { return };
    for sel in SELECTIONS {
        let targets: Vec<_> = devices(st, Some(focus)).filter(|d| !d.1.control() && d.1.serves(sel)).collect();
        for (_, proto, device) in targets {
            offer(st, focus, proto, device, sel);
        }
    }
}
