#![forbid(unsafe_code)]

use backend::{
    Event, Flip, Gesture, GestureKind, InputEvent, Mode, OutputEvent, OutputId, OutputInfo, Scroll, ScrollAxis,
};
use drm::Device as _;
use drm::control::{
    AtomicCommitFlags, Device as _, Event as DrmEvent, FbCmd2Flags, Mode as DrmMode, ModeTypeFlags, ResourceHandle,
    ResourceHandles, atomic::AtomicModeReq, connector, crtc, framebuffer, plane, property,
};
use gbm::{BufferObject, BufferObjectFlags, Modifier};
use input::{
    DeviceCapability, Libinput, LibinputInterface, SendEventsMode,
    event::{
        DeviceEvent, EventTrait, GestureEvent, KeyboardEvent, PointerEvent,
        gesture::{
            GestureEndEvent, GestureEventCoordinates, GestureEventTrait, GestureHoldEvent, GesturePinchEvent,
            GesturePinchEventTrait, GestureSwipeEvent,
        },
        keyboard::{KeyState, KeyboardEventTrait},
        pointer::{Axis, ButtonState, PointerEventTrait, PointerScrollEvent},
    },
};
use libseat::{Seat, SeatEvent};
use rustix::{
    event::{PollFd, PollFlags, Timespec},
    net::{AddressFamily, SocketFlags, SocketType, netlink},
};
use std::{
    cell::RefCell,
    collections::{HashMap, HashSet},
    error::Error,
    fs,
    os::fd::{AsFd, AsRawFd, BorrowedFd, OwnedFd, RawFd},
    path::{Path, PathBuf},
    rc::Rc,
};

mod planes;

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

const XRGB8888: u32 = 0x3432_5258;
const BUFFERS: usize = 2;

pub use input::{ClickMethod, ScrollMethod};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum DeviceKind {
    Keyboard,
    Pointer,
    Touchpad,
    Other,
}

#[derive(Clone, Copy, Debug)]
pub struct InputDevice<'a> {
    pub name: &'a str,
    pub kind: DeviceKind,
    pub vendor: u32,
    pub product: u32,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Accel {
    Flat(f64),
    Adaptive(f64),
}

#[derive(Clone, Copy, Debug, Default)]
pub struct InputConfig {
    pub disabled: bool,
    pub tap: Option<bool>,
    pub tap_drag: Option<bool>,
    pub drag_lock: Option<bool>,
    pub natural_scroll: Option<bool>,
    pub left_handed: Option<bool>,
    pub middle_emulation: Option<bool>,
    pub disable_while_typing: Option<bool>,
    pub accel: Option<Accel>,
    pub scroll_method: Option<ScrollMethod>,
    pub scroll_button: Option<u32>,
    pub click_method: Option<ClickMethod>,
    pub scroll_factor: Option<f64>,
}

pub type Config = fn(&InputDevice) -> InputConfig;

struct Card(OwnedFd);

impl AsFd for Card {
    fn as_fd(&self) -> BorrowedFd<'_> {
        self.0.as_fd()
    }
}

impl drm::Device for Card {}
impl drm::control::Device for Card {}

#[derive(Default)]
struct Session {
    seat: Option<Seat>,
    devices: HashMap<RawFd, libseat::Device>,
    changes: Rc<RefCell<Vec<bool>>>,
}

impl Session {
    fn open(&mut self, path: &Path) -> std::result::Result<OwnedFd, i32> {
        let device = self.seat.as_mut().ok_or(-1)?.open_device(&path).map_err(|e| -e.0)?;
        let fd = device.as_fd().try_clone_to_owned().map_err(|_| -1)?;
        self.devices.insert(fd.as_raw_fd(), device);
        Ok(fd)
    }
}

struct Interface(Rc<RefCell<Session>>);

impl LibinputInterface for Interface {
    fn open_restricted(&mut self, path: &Path, _: i32) -> std::result::Result<OwnedFd, i32> {
        self.0.borrow_mut().open(path)
    }

    fn close_restricted(&mut self, fd: OwnedFd) {
        let mut session = self.0.borrow_mut();
        if let Some(device) = session.devices.remove(&fd.as_raw_fd())
            && let Some(seat) = session.seat.as_mut()
        {
            let _ = seat.close_device(device);
        }
    }
}

pub struct Scanout {
    pub dmabuf: gfx::Dmabuf,
    pub serial: u64,
    framebuffer: framebuffer::Handle,
    _bo: BufferObject<()>,
}

struct Output {
    id: OutputId,
    name: String,
    connector: connector::Handle,
    crtc: crtc::Handle,
    plane: plane::Handle,
    modes: Vec<DrmMode>,
    mode: DrmMode,
    mode_blob: u64,
    edid: Vec<u8>,
    properties: HashMap<(u32, &'static str), property::Handle>,
    buffers: Vec<Scanout>,
    front: Option<Shown>,
    pending: Option<Shown>,
    formats: Vec<(u32, u64)>,
    cursor_plane: Option<plane::Handle>,
    cursor: Option<planes::CursorPlane>,
    committed_cursor: Option<(framebuffer::Handle, (i32, i32))>,
    gamma_size: Option<u32>,
    pending_gamma: Option<Option<Vec<u16>>>,
    gamma_blob: u32,
    modeset: bool,
    enabled: bool,
    vrr: bool,
}

impl Output {
    fn info(&self) -> OutputInfo {
        let modes = self.modes.iter().map(mode).collect();
        let preferred = self.modes.iter().find(|m| m.mode_type().contains(ModeTypeFlags::PREFERRED)).map(mode);
        let [make, model, serial] = edid_identity(&self.edid);
        OutputInfo {
            id: self.id,
            name: self.name.clone(),
            make,
            model,
            serial,
            modes,
            preferred,
            mode: mode(&self.mode),
            scale: 1.0,
        }
    }
}

pub struct Kms {
    config: Config,
    session: Rc<RefCell<Session>>,
    card: Card,
    gbm: gbm::Device<Card>,
    input: Option<Libinput>,
    seat_fd: Option<OwnedFd>,
    uevents: Option<OwnedFd>,
    formats: Vec<(u32, u64)>,
    outputs: Vec<Output>,
    events: Vec<Event>,
    active: bool,
    next_id: OutputId,
    next_serial: u64,
    clients: HashMap<u64, planes::ClientFb>,
    handles: planes::Handles,
    rejected: HashSet<u64>,
    dead: Vec<u64>,
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Shown {
    Slot(usize),
    Client(u64),
}

pub enum Primary<'a> {
    Slot(usize, Option<OwnedFd>),
    Client(u64, &'a gfx::Dmabuf),
    Keep,
}

#[derive(Clone, Copy)]
pub struct CursorImage<'a> {
    pub key: u64,
    pub pixels: &'a [u32],
    pub size: (i32, i32),
    pub position: (i32, i32),
}

impl Kms {
    pub fn open(config: Config, render_node: u64, formats: &[(u32, u64)], use_seat: bool) -> Result<Self> {
        let session = Rc::new(RefCell::new(Session::default()));
        let path = card_path(render_node)?;
        let card = match use_seat {
            true => Card(open_with_seat(&session, &path)?),
            false => Card(rustix::fs::open(&path, rustix::fs::OFlags::RDWR | rustix::fs::OFlags::CLOEXEC, 0.into())?),
        };
        rustix::fs::fcntl_setfl(&card.0, rustix::fs::fcntl_getfl(&card.0)? | rustix::fs::OFlags::NONBLOCK)?;
        card.set_client_capability(drm::ClientCapability::UniversalPlanes, true)?;
        card.set_client_capability(drm::ClientCapability::Atomic, true)?;
        let gbm = gbm::Device::new(Card(card.0.try_clone()?))?;
        let input = use_seat.then(|| open_input(&session)).transpose()?;
        let seat_fd = session.borrow_mut().seat.as_mut().and_then(|s| s.get_fd().ok()?.try_clone_to_owned().ok());
        let uevents = uevent_socket().inspect_err(|e| log::warn!("no hotplug events: {e}")).ok();
        let (outputs, events, formats) = (vec![], vec![], formats.to_vec());
        let mut kms = Self {
            config,
            session,
            card,
            gbm,
            input,
            seat_fd,
            uevents,
            formats,
            outputs,
            events,
            active: true,
            next_id: 0,
            next_serial: 0,
            clients: HashMap::new(),
            handles: HashMap::new(),
            rejected: HashSet::new(),
            dead: vec![],
        };
        kms.scan()?;
        Ok(kms)
    }

    pub fn fds(&self) -> Vec<BorrowedFd<'_>> {
        let mut fds = vec![self.card.as_fd()];
        fds.extend(self.input.as_ref().map(|i| i.as_fd()));
        fds.extend(self.seat_fd.as_ref().map(|fd| fd.as_fd()));
        fds.extend(self.uevents.as_ref().map(|fd| fd.as_fd()));
        fds
    }

    fn output(&self, id: OutputId) -> Option<&Output> {
        self.outputs.iter().find(|o| o.id == id)
    }

    pub fn size(&self, id: OutputId) -> (i32, i32) {
        self.output(id).map_or((0, 0), |o| (o.mode.size().0.into(), o.mode.size().1.into()))
    }

    pub fn ready(&self, id: OutputId) -> bool {
        self.active && self.output(id).is_some_and(|o| o.enabled && o.pending.is_none() && !o.buffers.is_empty())
    }

    pub fn next_buffer(&self, id: OutputId) -> Option<(usize, &Scanout)> {
        let o = self.output(id)?;
        let free = |i: usize| ![o.front, o.pending].contains(&Some(Shown::Slot(i)));
        (0..o.buffers.len()).find(|&i| free(i)).map(|i| (i, &o.buffers[i]))
    }

    pub fn configure(&mut self, id: OutputId, wanted: Option<Mode>, vrr: bool, enabled: bool) -> Result<()> {
        let Some(index) = self.outputs.iter().position(|o| o.id == id) else { return Ok(()) };
        let o = &self.outputs[index];
        let chosen = choose(o, wanted);
        let resized = chosen.size() != o.mode.size() || o.buffers.is_empty();
        let changed = mode(&chosen) != mode(&o.mode);
        let modeset = resized || changed || vrr != o.vrr || !o.enabled;
        if !enabled {
            (self.outputs[index].mode, self.outputs[index].vrr) = (chosen, vrr);
            if self.outputs[index].enabled {
                self.disable(index)?;
            }
            return Ok(());
        }
        let buffers = match resized {
            true => Some(allocate(&self.card, &self.gbm, o, chosen, &self.formats)?),
            false => None,
        };
        if modeset && self.active {
            let framebuffer = buffers.as_ref().unwrap_or(&o.buffers)[0].framebuffer;
            if let Err(e) = test_mode(&self.card, o, chosen, vrr, framebuffer) {
                buffers.into_iter().flatten().for_each(|b| drop(self.card.destroy_framebuffer(b.framebuffer)));
                return Err(format!("mode {:?} refused: {e}", mode(&chosen)).into());
            }
        }
        let o = &mut self.outputs[index];
        (o.mode, o.vrr) = (chosen, vrr);
        if let Some(buffers) = buffers {
            self.install(index, buffers);
        }
        let o = &mut self.outputs[index];
        o.enabled = true;
        o.modeset |= modeset;
        if changed {
            let info = o.info();
            self.events.push(Event::Output(OutputEvent::Changed(info)));
        }
        Ok(())
    }

    pub fn test(&self, id: OutputId, wanted: Option<Mode>, vrr: bool) -> Result<()> {
        let Some(o) = self.outputs.iter().find(|o| o.id == id) else { return Ok(()) };
        let chosen = choose(o, wanted);
        if !self.active || (o.enabled && mode(&chosen) == mode(&o.mode) && vrr == o.vrr) {
            return Ok(());
        }
        let buffers = match chosen.size() != o.mode.size() || o.buffers.is_empty() {
            true => Some(allocate(&self.card, &self.gbm, o, chosen, &self.formats)?),
            false => None,
        };
        let result = test_mode(&self.card, o, chosen, vrr, buffers.as_ref().unwrap_or(&o.buffers)[0].framebuffer);
        buffers.into_iter().flatten().for_each(|b| drop(self.card.destroy_framebuffer(b.framebuffer)));
        result.map_err(|e| format!("mode {:?} refused: {e}", mode(&chosen)).into())
    }

    pub fn present(
        &mut self,
        id: OutputId,
        primary: Primary,
        cursor: Option<CursorImage>,
        tearing: bool,
    ) -> Result<()> {
        let index = self.outputs.iter().position(|o| o.id == id).ok_or("unknown output")?;
        let (framebuffer, shown, fence) = self.primary(index, primary)?;
        let in_fence = fence.as_ref().map(|f| property::Value::SignedRange(f.as_raw_fd().into()));
        let modeset = self.outputs[index].modeset;
        let others = if modeset { self.foreign_planes(index) } else { vec![] };
        let o = &mut self.outputs[index];
        let cursor = match (cursor, o.cursor.as_mut()) {
            (Some(c), Some(plane)) => Some((plane.framebuffer(c.key, c.pixels, c.size)?, c.position)),
            _ => None,
        };
        let gamma = o.pending_gamma.take();
        if tearing && !modeset && cursor == o.committed_cursor && gamma.is_none() {
            match flip_async(&self.card, o, framebuffer, in_fence) {
                Ok(()) => {
                    o.pending = Some(shown);
                    return Ok(());
                }
                Err(e) => log::debug!("async flip refused, flipping on vblank: {e}"),
            }
        }
        let mut request = Request::new(o);
        let mut flags = AtomicCommitFlags::PAGE_FLIP_EVENT | AtomicCommitFlags::NONBLOCK;
        let mut blobs = vec![];
        if modeset {
            let blob = match mode_blob(&self.card, &o.mode) {
                Ok(blob) => blob,
                Err(e) => {
                    o.pending_gamma = gamma;
                    return Err(e);
                }
            };
            blobs.push(blob);
            request.modeset(o, blob, o.vrr);
            request.set(o.crtc, "DEGAMMA_LUT", property::Value::Blob(0));
            request.set(o.crtc, "CTM", property::Value::Blob(0));
            request.set(o.crtc, "GAMMA_LUT", property::Value::Blob(o.gamma_blob.into()));
            for &(plane, fb, crtc) in &others {
                request.0.add_property(plane, fb, property::Value::Framebuffer(None));
                request.0.add_property(plane, crtc, property::Value::CRTC(None));
            }
            flags |= AtomicCommitFlags::ALLOW_MODESET;
        }
        let (w, h) = o.mode.size();
        request.place(o.plane, Some((framebuffer, o.crtc)), (0, 0), (w.into(), h.into()));
        if let Some(fence) = in_fence {
            request.set(o.plane, "IN_FENCE_FD", fence);
        }
        if let Some(plane) = &o.cursor {
            let (fb, position) = cursor.map_or((None, (0, 0)), |(fb, position)| (Some((fb, o.crtc)), position));
            request.place(plane.plane, fb, position, plane.size);
        }
        let gamma_blob = match &gamma {
            Some(ramps) => match gamma_blob(&self.card, o.gamma_size, ramps.as_deref()) {
                Ok(blob) => Some(blob),
                Err(e) => {
                    log::warn!("cannot set gamma on output {}: {e}", o.id);
                    self.events.push(Event::Output(OutputEvent::GammaFailed(o.id)));
                    None
                }
            },
            None => None,
        };
        if let Some(blob) = gamma_blob {
            request.set(o.crtc, "GAMMA_LUT", property::Value::Blob(blob.into()));
            blobs.extend(Some(u64::from(blob)).filter(|&b| b != 0));
        }
        if let Err(e) = self.card.atomic_commit(flags, request.0) {
            let errno = e.raw_os_error().map(rustix::io::Errno::from_raw_os_error);
            let invalid = matches!(errno, Some(rustix::io::Errno::INVAL | rustix::io::Errno::RANGE));
            if let Shown::Client(key) = shown
                && invalid
                && !modeset
            {
                self.rejected.insert(key);
            }
            blobs.into_iter().for_each(|b| drop(self.card.destroy_property_blob(b)));
            match gamma_blob.is_some() && invalid {
                true => self.events.push(Event::Output(OutputEvent::GammaFailed(o.id))),
                false => o.pending_gamma = o.pending_gamma.take().or(gamma),
            }
            return Err(e.into());
        }
        if let Some(blob) = gamma_blob {
            let old = std::mem::replace(&mut o.gamma_blob, blob);
            if old != 0 {
                let _ = self.card.destroy_property_blob(old.into());
            }
        }
        if modeset {
            let old = std::mem::replace(&mut o.mode_blob, blobs[0]);
            if old != 0 {
                let _ = self.card.destroy_property_blob(old);
            }
        }
        if let Some(plane) = o.cursor.as_mut() {
            plane.committed(cursor.is_some());
        }
        (o.pending, o.modeset, o.committed_cursor) = (Some(shown), false, cursor);
        Ok(())
    }

    fn foreign_planes(&self, index: usize) -> Vec<(plane::Handle, property::Handle, property::Handle)> {
        let o = &self.outputs[index];
        let ours: Vec<plane::Handle> =
            self.outputs.iter().flat_map(|o| [Some(o.plane), o.cursor.as_ref().map(|c| c.plane)]).flatten().collect();
        let planes = self.card.plane_handles().unwrap_or_default();
        let attached = |p: &plane::Handle| self.card.get_plane(*p).is_ok_and(|i| i.crtc() == Some(o.crtc));
        let handles =
            |p: plane::Handle| Some((p, property(&self.card, p, "FB_ID")?.0, property(&self.card, p, "CRTC_ID")?.0));
        planes.into_iter().filter(|p| !ours.contains(p) && attached(p)).filter_map(handles).collect()
    }

    fn primary(&mut self, index: usize, primary: Primary) -> Result<(framebuffer::Handle, Shown, Option<OwnedFd>)> {
        let (shown, fence) = match primary {
            Primary::Slot(slot, fence) => (Shown::Slot(slot), fence),
            Primary::Client(key, dmabuf) => {
                if !self.clients.contains_key(&key) {
                    let fb = planes::import(&self.card, dmabuf, &mut self.handles).inspect_err(|_| {
                        self.rejected.insert(key);
                    })?;
                    self.clients.insert(key, fb);
                }
                (Shown::Client(key), None)
            }
            Primary::Keep => (self.outputs[index].front.ok_or("nothing on screen")?, None),
        };
        let o = &self.outputs[index];
        let framebuffer = match shown {
            Shown::Slot(slot) => o.buffers.get(slot).ok_or("unknown buffer")?.framebuffer,
            Shown::Client(key) => self.clients.get(&key).ok_or("unknown client buffer")?.framebuffer,
        };
        let fence = match (fence, shown) {
            (Some(fence), Shown::Slot(slot)) if !o.properties.contains_key(&(o.plane.into(), "IN_FENCE_FD")) => {
                gfx::attach_fence(&o.buffers[slot].dmabuf, fence.as_fd());
                None
            }
            (fence, _) => fence,
        };
        Ok((framebuffer, shown, fence))
    }

    pub fn can_scanout(&self, id: OutputId, key: u64, dmabuf: &gfx::Dmabuf) -> bool {
        self.output(id).is_some_and(|o| {
            let (w, h) = o.mode.size();
            dmabuf.size == (w.into(), h.into()) && o.formats.contains(&(dmabuf.format, dmabuf.modifier))
        }) && !self.rejected.contains(&key)
    }

    pub fn cursor_size(&self, id: OutputId) -> Option<(i32, i32)> {
        let plane = self.output(id)?.cursor.as_ref()?;
        Some((plane.size.0 as i32, plane.size.1 as i32))
    }

    pub fn gamma_size(&self, id: OutputId) -> Option<u32> {
        self.output(id)?.gamma_size
    }

    pub fn set_gamma(&mut self, id: OutputId, ramps: Option<Vec<u16>>) {
        if let Some(o) = self.outputs.iter_mut().find(|o| o.id == id) {
            o.pending_gamma = Some(ramps);
        }
    }

    pub fn needs_commit(&self, id: OutputId) -> bool {
        self.output(id).is_some_and(|o| o.pending_gamma.is_some())
    }

    pub fn has_front(&self, id: OutputId) -> bool {
        self.output(id).is_some_and(|o| o.front.is_some())
    }

    pub fn scanned_out(&self) -> Vec<u64> {
        let shown = self.outputs.iter().flat_map(|o| [o.front, o.pending]).flatten();
        shown.filter_map(|s| if let Shown::Client(key) = s { Some(key) } else { None }).collect()
    }

    pub fn forget_clients(&mut self, dead: impl IntoIterator<Item = u64>) {
        self.dead.extend(dead);
        if self.dead.is_empty() {
            return;
        }
        let busy = self.scanned_out();
        for key in self.dead.extract_if(.., |k| !busy.contains(k)) {
            if let Some(fb) = self.clients.remove(&key) {
                planes::release(&self.card, fb, &mut self.handles);
            }
            self.rejected.remove(&key);
        }
    }

    fn disable(&mut self, index: usize) -> Result<()> {
        let o = &mut self.outputs[index];
        let mut request = Request::new(o);
        request.set(o.connector, "CRTC_ID", property::Value::CRTC(None));
        request.set(o.crtc, "ACTIVE", property::Value::Boolean(false));
        request.set(o.crtc, "MODE_ID", property::Value::Blob(0));
        for plane in [Some(o.plane), o.cursor.as_ref().map(|c| c.plane)].into_iter().flatten() {
            request.place(plane, None, (0, 0), (0, 0));
        }
        let request = request.0;
        o.committed_cursor = None;
        (o.enabled, o.front, o.pending) = (false, None, None);
        if self.active {
            self.card.atomic_commit(AtomicCommitFlags::ALLOW_MODESET, request)?;
        }
        let blob = std::mem::take(&mut o.mode_blob);
        if blob != 0 {
            let _ = self.card.destroy_property_blob(blob);
        }
        Ok(())
    }

    fn install(&mut self, index: usize, mut buffers: Vec<Scanout>) {
        let o = &mut self.outputs[index];
        destroy_buffers(&self.card, o);
        if o.cursor.is_none()
            && let Some(plane) = o.cursor_plane
        {
            let cap = |c| self.card.get_driver_capability(c).ok().map(|v| v as u32);
            let size = (
                cap(drm::DriverCapability::CursorWidth).unwrap_or(64),
                cap(drm::DriverCapability::CursorHeight).unwrap_or(64),
            );
            o.cursor = planes::CursorPlane::new(&self.card, &self.gbm, plane, size)
                .inspect_err(|e| log::warn!("{}: no cursor plane: {e}", o.name))
                .ok();
        }
        for buffer in &mut buffers {
            self.next_serial += 1;
            buffer.serial = self.next_serial;
        }
        (o.buffers, o.front, o.pending, o.modeset) = (buffers, None, None, true);
    }

    pub fn switch_vt(&mut self, vt: i32) {
        if let Some(seat) = self.session.borrow_mut().seat.as_mut()
            && let Err(e) = seat.switch_session(vt)
        {
            log::warn!("cannot switch to vt {vt}: {e}");
        }
    }

    pub fn step_vt(&mut self, delta: i32) {
        let active = std::fs::read_to_string("/sys/class/tty/tty0/active").unwrap_or_default();
        match active.trim().strip_prefix("tty").and_then(|n| n.parse::<i32>().ok()) {
            Some(vt) => self.switch_vt((vt - 1 + delta).rem_euclid(12) + 1),
            None => log::warn!("cannot read the active vt from {active:?}"),
        }
    }

    pub fn dispatch(&mut self, readable: bool, events: &mut Vec<Event>) -> Result<()> {
        events.append(&mut self.events);
        if !readable {
            return Ok(());
        }
        let ready = self.readable();
        if ready[2] {
            self.dispatch_seat(events);
        }
        if ready[0]
            && let Ok(drm_events) = self.card.receive_events()
        {
            for event in drm_events {
                let DrmEvent::PageFlip(flip) = event else { continue };
                if let Some(o) = self.outputs.iter_mut().find(|o| o.crtc == flip.crtc) {
                    o.front = o.pending.take().or(o.front);
                    let flip = Flip { time: flip.duration, sequence: flip.frame.into() };
                    events.push(Event::Frame { output: o.id, flip: Some(flip) });
                }
            }
        }
        if ready[3] && self.hotplugged() {
            self.scan()?;
            events.append(&mut self.events);
        }
        match ready[1] {
            true => self.dispatch_input(events),
            false => Ok(()),
        }
    }

    fn readable(&self) -> [bool; 4] {
        let sources = [
            Some(self.card.as_fd()),
            self.input.as_ref().map(|i| i.as_fd()),
            self.seat_fd.as_ref().map(|fd| fd.as_fd()),
            self.uevents.as_ref().map(|fd| fd.as_fd()),
        ];
        let mut fds: Vec<PollFd> = sources.iter().flatten().map(|fd| PollFd::new(fd, PollFlags::IN)).collect();
        if rustix::event::poll(&mut fds, Some(&Timespec { tv_sec: 0, tv_nsec: 0 })).is_err() {
            return [true; 4];
        }
        let mut ready = fds.iter().map(|fd| !fd.revents().is_empty());
        sources.map(|fd| fd.is_some() && ready.next().unwrap_or(false))
    }

    fn hotplugged(&self) -> bool {
        let Some(socket) = &self.uevents else { return false };
        let mut buffer = [0; 8192];
        let mut drm = false;
        while let Ok(n) = rustix::io::read(socket, &mut buffer) {
            drm |= buffer[..n].split(|&b| b == 0).any(|field| field == b"SUBSYSTEM=drm");
        }
        drm
    }

    fn scan(&mut self) -> Result<()> {
        let resources = self.card.resource_handles()?;
        let connected: Vec<connector::Info> = resources
            .connectors()
            .iter()
            .filter_map(|&c| self.card.get_connector(c, true).ok())
            .filter(|c| c.state() == connector::State::Connected && !c.modes().is_empty())
            .collect();
        for index in (0..self.outputs.len()).rev() {
            let o = &self.outputs[index];
            let same = |c: &&connector::Info| {
                c.handle() == o.connector && c.modes() == o.modes && edid(&self.card, c.handle()) == o.edid
            };
            if !connected.iter().any(|c| same(&c)) {
                let _ = self.disable(index);
                let output = self.outputs.remove(index);
                self.events.push(Event::Output(OutputEvent::Removed(output.id)));
                release(&self.card, output);
            }
        }
        for info in connected {
            if self.outputs.iter().any(|o| o.connector == info.handle()) {
                continue;
            }
            let used: Vec<_> = self.outputs.iter().map(|o| o.crtc).collect();
            match new_output(&self.card, &resources, &info, &used) {
                Ok(mut output) => {
                    self.next_id += 1;
                    output.id = self.next_id;
                    let info = output.info();
                    log::info!(
                        "output {} {} ({} {} {}): {:?}",
                        info.id,
                        info.name,
                        info.make,
                        info.model,
                        info.serial,
                        info.mode
                    );
                    self.events.push(Event::Output(OutputEvent::Added(info)));
                    self.outputs.push(output);
                }
                Err(e) => log::warn!("cannot use connector {:?}: {e}", info.handle()),
            }
        }
        Ok(())
    }

    fn dispatch_seat(&mut self, events: &mut Vec<Event>) {
        let changes = {
            let mut session = self.session.borrow_mut();
            if let Some(seat) = session.seat.as_mut() {
                let _ = seat.dispatch(0);
            }
            std::mem::take(&mut *session.changes.borrow_mut())
        };
        for active in changes {
            self.active = active;
            match (active, self.input.as_mut()) {
                (true, Some(input)) => drop(input.resume()),
                (false, Some(input)) => input.suspend(),
                _ => {}
            }
            for o in &mut self.outputs {
                (o.front, o.pending, o.modeset) = (None, None, o.modeset || active);
            }
            events.push(Event::Session(active));
        }
    }

    fn dispatch_input(&mut self, events: &mut Vec<Event>) -> Result<()> {
        let Some(input) = self.input.as_mut() else { return Ok(()) };
        input.dispatch()?;
        let mut push = |event| events.push(Event::Input(event));
        for event in input.by_ref() {
            match event {
                input::Event::Device(DeviceEvent::Added(added)) => configure_device(added.device(), self.config),
                input::Event::Keyboard(KeyboardEvent::Key(key)) => {
                    let pressed = key.key_state() == KeyState::Pressed;
                    if key.seat_key_count() == u32::from(pressed) {
                        push(InputEvent::Key { key: key.key(), pressed })
                    }
                }
                input::Event::Pointer(PointerEvent::Motion(m)) => {
                    let delta = [m.dx(), m.dy(), m.dx_unaccelerated(), m.dy_unaccelerated()];
                    push(InputEvent::RelativeMotion { delta, usec: m.time_usec() });
                }
                input::Event::Pointer(PointerEvent::Button(b)) => {
                    let pressed = b.button_state() == ButtonState::Pressed;
                    if b.seat_button_count() == u32::from(pressed) {
                        push(InputEvent::Button { button: b.button(), pressed })
                    }
                }
                input::Event::Pointer(PointerEvent::ScrollWheel(s)) => {
                    push(scroll(&s, self.config, Scroll::WHEEL, |axis| s.scroll_value_v120(axis)))
                }
                input::Event::Pointer(PointerEvent::ScrollFinger(s)) => {
                    push(scroll(&s, self.config, Scroll::FINGER, |_| 0.0))
                }
                input::Event::Pointer(PointerEvent::ScrollContinuous(s)) => {
                    push(scroll(&s, self.config, Scroll::CONTINUOUS, |_| 0.0))
                }
                input::Event::Gesture(g) => gesture(g).into_iter().for_each(|g| push(InputEvent::Gesture(g))),
                _ => {}
            }
        }
        Ok(())
    }

    pub fn outputs(&self) -> Vec<OutputInfo> {
        self.outputs.iter().map(Output::info).collect()
    }
}

fn mode(m: &DrmMode) -> Mode {
    let (_, _, htotal) = m.hsync();
    let (_, _, vtotal) = m.vsync();
    let refresh = u64::from(m.clock()) * 1_000_000 / (u64::from(htotal) * u64::from(vtotal)).max(1);
    Mode { width: m.size().0.into(), height: m.size().1.into(), refresh: refresh as u32 }
}

fn choose(o: &Output, wanted: Option<Mode>) -> DrmMode {
    wanted.and_then(|w| o.modes.iter().copied().find(|m| close(&mode(m), &w))).unwrap_or(o.mode)
}

fn close(a: &Mode, b: &Mode) -> bool {
    (a.width, a.height) == (b.width, b.height) && a.refresh.abs_diff(b.refresh) < 500
}

fn gesture(event: GestureEvent) -> Option<Gesture> {
    let fingers = |e: &dyn GestureEventTrait| e.finger_count().max(0) as u32;
    let (swipe, pinch, hold) = (GestureKind::Swipe, GestureKind::Pinch, GestureKind::Hold);
    Some(match event {
        GestureEvent::Swipe(GestureSwipeEvent::Begin(e)) => Gesture::Begin(swipe, fingers(&e)),
        GestureEvent::Swipe(GestureSwipeEvent::Update(e)) => {
            Gesture::Update { dx: e.dx(), dy: e.dy(), scale: 1.0, rotation: 0.0 }
        }
        GestureEvent::Swipe(GestureSwipeEvent::End(e)) => Gesture::End(e.cancelled()),
        GestureEvent::Pinch(GesturePinchEvent::Begin(e)) => Gesture::Begin(pinch, fingers(&e)),
        GestureEvent::Pinch(GesturePinchEvent::Update(e)) => {
            Gesture::Update { dx: e.dx(), dy: e.dy(), scale: e.scale(), rotation: e.angle_delta() }
        }
        GestureEvent::Pinch(GesturePinchEvent::End(e)) => Gesture::End(e.cancelled()),
        GestureEvent::Hold(GestureHoldEvent::Begin(e)) => Gesture::Begin(hold, fingers(&e)),
        GestureEvent::Hold(GestureHoldEvent::End(e)) => Gesture::End(e.cancelled()),
        _ => return None,
    })
}

fn describe(device: &input::Device) -> InputDevice<'_> {
    let has = |cap| device.has_capability(cap);
    let kind = match () {
        _ if has(DeviceCapability::Pointer) && device.config_tap_finger_count() > 0 => DeviceKind::Touchpad,
        _ if has(DeviceCapability::Pointer) => DeviceKind::Pointer,
        _ if has(DeviceCapability::Keyboard) => DeviceKind::Keyboard,
        _ => DeviceKind::Other,
    };
    InputDevice { name: device.name(), kind, vendor: device.id_vendor(), product: device.id_product() }
}

fn scroll<E: PointerScrollEvent + EventTrait>(
    event: &E,
    config: Config,
    source: u32,
    value120: impl Fn(Axis) -> f64,
) -> InputEvent {
    let device = event.device();
    let factor = config(&describe(&device)).scroll_factor.unwrap_or(1.0);
    let inverted = device.config_scroll_natural_scroll_enabled();
    let axis = |axis| {
        event.has_axis(axis).then(|| ScrollAxis {
            value: event.scroll_value(axis) * factor,
            value120: (value120(axis) * factor).round() as i32,
            inverted,
        })
    };
    InputEvent::Scroll(Scroll { source, axes: [axis(Axis::Vertical), axis(Axis::Horizontal)] })
}

fn configure_device(mut device: input::Device, config: Config) {
    let c = config(&describe(&device));
    log::info!("input device {:?}: {c:?}", describe(&device));
    let mode = if c.disabled { SendEventsMode::DISABLED } else { SendEventsMode::ENABLED };
    let _ = device.config_send_events_set_mode(mode);
    c.tap.map(|v| device.config_tap_set_enabled(v));
    c.tap_drag.map(|v| device.config_tap_set_drag_enabled(v));
    c.drag_lock.map(|v| device.config_tap_set_drag_lock_enabled(v));
    c.natural_scroll.map(|v| device.config_scroll_set_natural_scroll_enabled(v));
    c.left_handed.map(|v| device.config_left_handed_set(v));
    c.middle_emulation.map(|v| device.config_middle_emulation_set_enabled(v));
    c.disable_while_typing.map(|v| device.config_dwt_set_enabled(v));
    c.scroll_method.map(|v| device.config_scroll_set_method(v));
    c.scroll_button.map(|v| device.config_scroll_set_button(v));
    c.click_method.map(|v| device.config_click_set_method(v));
    if let Some(accel) = c.accel {
        let (profile, speed) = match accel {
            Accel::Flat(speed) => (input::AccelProfile::Flat, speed),
            Accel::Adaptive(speed) => (input::AccelProfile::Adaptive, speed),
        };
        let _ = device.config_accel_set_profile(profile);
        let _ = device.config_accel_set_speed(speed);
    }
}

fn card_path(render_node: u64) -> Result<PathBuf> {
    let (major, minor) = (rustix::fs::major(render_node), rustix::fs::minor(render_node));
    let entries = fs::read_dir(format!("/sys/dev/char/{major}:{minor}/device/drm"))?;
    let card = entries.flatten().map(|e| e.file_name()).find(|n| n.to_string_lossy().starts_with("card"));
    Ok(Path::new("/dev/dri").join(card.ok_or("no card node for the render device")?))
}

fn uevent_socket() -> Result<OwnedFd> {
    let flags = SocketFlags::CLOEXEC | SocketFlags::NONBLOCK;
    let protocol = Some(netlink::KOBJECT_UEVENT);
    let socket = rustix::net::socket_with(AddressFamily::NETLINK, SocketType::DGRAM, flags, protocol)?;
    rustix::net::bind(&socket, &netlink::SocketAddrNetlink::new(0, 1))?;
    Ok(socket)
}

fn open_with_seat(session: &Rc<RefCell<Session>>, path: &Path) -> Result<OwnedFd> {
    let changes = session.borrow().changes.clone();
    let seat = Seat::open(move |seat, event| {
        if let SeatEvent::Disable = event {
            let _ = seat.disable();
        }
        changes.borrow_mut().push(matches!(event, SeatEvent::Enable));
    })?;
    let mut s = session.borrow_mut();
    s.seat = Some(seat);
    let fd = s.open(path).map_err(|e| std::io::Error::from_raw_os_error(-e))?;
    s.changes.borrow_mut().clear();
    Ok(fd)
}

fn open_input(session: &Rc<RefCell<Session>>) -> Result<Libinput> {
    let mut input = Libinput::new_with_udev(Interface(session.clone()));
    input.udev_assign_seat("seat0").map_err(|_| "cannot assign libinput seat")?;
    Ok(input)
}

fn new_output(
    card: &Card,
    resources: &ResourceHandles,
    info: &connector::Info,
    used: &[crtc::Handle],
) -> Result<Output> {
    let preferred = info.modes().iter().find(|m| m.mode_type().contains(ModeTypeFlags::PREFERRED));
    let mode = *preferred.unwrap_or(&info.modes()[0]);
    let current = info.current_encoder().and_then(|e| card.get_encoder(e).ok()?.crtc());
    let crtc = current
        .into_iter()
        .chain(
            info.encoders()
                .iter()
                .filter_map(|&e| card.get_encoder(e).ok())
                .flat_map(|e| resources.filter_crtcs(e.possible_crtcs())),
        )
        .find(|c| !used.contains(c))
        .ok_or("no free crtc")?;
    let planes = card.plane_handles()?;
    let find_plane = |kind| {
        planes.iter().copied().find(|&p| {
            card.get_plane(p).is_ok_and(|i| resources.filter_crtcs(i.possible_crtcs()).contains(&crtc))
                && property_value(card, p, "type") == Some(kind)
        })
    };
    let (plane, cursor_plane) = (find_plane(1).ok_or("no primary plane")?, find_plane(2));
    let mut properties = HashMap::new();
    collect_properties(card, info.handle(), &["CRTC_ID"], &mut properties);
    collect_properties(
        card,
        crtc,
        &["MODE_ID", "ACTIVE", "VRR_ENABLED", "GAMMA_LUT", "DEGAMMA_LUT", "CTM"],
        &mut properties,
    );
    let plane_properties =
        ["FB_ID", "CRTC_ID", "SRC_X", "SRC_Y", "SRC_W", "SRC_H", "CRTC_X", "CRTC_Y", "CRTC_W", "CRTC_H"];
    collect_properties(card, plane, &plane_properties, &mut properties);
    collect_properties(card, plane, &["IN_FENCE_FD"], &mut properties);
    if let Some(cursor) = cursor_plane {
        collect_properties(card, cursor, &plane_properties, &mut properties);
    }
    let name = format!("{}-{}", info.interface().as_str(), info.interface_id());
    Ok(Output {
        id: 0,
        name,
        connector: info.handle(),
        crtc,
        plane,
        modes: info.modes().to_vec(),
        mode,
        mode_blob: 0,
        edid: edid(card, info.handle()),
        properties,
        buffers: vec![],
        front: None,
        pending: None,
        formats: planes::formats(card, plane),
        cursor_plane,
        cursor: None,
        committed_cursor: None,
        gamma_size: property_value(card, crtc, "GAMMA_LUT_SIZE").map(|s| s as u32).filter(|&s| s > 0),
        pending_gamma: None,
        gamma_blob: 0,
        modeset: true,
        enabled: false,
        vrr: false,
    })
}

fn edid(card: &Card, connector: connector::Handle) -> Vec<u8> {
    let blob = property_value(card, connector, "EDID").filter(|&b| b != 0);
    blob.and_then(|b| card.get_property_blob(b).ok()).unwrap_or_default()
}

fn edid_identity(edid: &[u8]) -> [String; 3] {
    let text = |tag| {
        [54, 72, 90, 108].into_iter().find_map(|at| {
            let descriptor = edid.get(at..at + 18)?;
            (descriptor[..3] == [0, 0, 0] && descriptor[3] == tag).then(|| {
                let text = &descriptor[5..];
                let end = text.iter().position(|&b| b == b'\n').unwrap_or(text.len());
                String::from_utf8_lossy(&text[..end]).trim().to_string()
            })
        })
    };
    let make = edid
        .get(8..10)
        .map(|b| u16::from_be_bytes([b[0], b[1]]))
        .filter(|&id| id != 0)
        .map(|id| [10, 5, 0].into_iter().map(|shift| char::from(b'@' + ((id >> shift) & 0x1f) as u8)).collect());
    let number = edid.get(12..16).map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]])).filter(|&n| n != 0);
    let serial = text(0xff).or_else(|| number.map(|n| format!("0x{n:08X}")));
    [make.unwrap_or_default(), text(0xfc).unwrap_or_default(), serial.unwrap_or_default()]
}

fn collect_properties(
    card: &Card,
    object: impl ResourceHandle,
    names: &[&'static str],
    properties: &mut HashMap<(u32, &'static str), property::Handle>,
) {
    for &name in names {
        if let Some((handle, _)) = property(card, object, name) {
            properties.insert((object.into(), name), handle);
        }
    }
}

fn property(card: &Card, object: impl ResourceHandle, name: &str) -> Option<(property::Handle, u64)> {
    let properties = card.get_properties(object).ok()?;
    let (handles, values) = properties.as_props_and_values();
    let named = |h: &property::Handle| card.get_property(*h).is_ok_and(|p| p.name().to_bytes() == name.as_bytes());
    handles.iter().position(named).map(|i| (handles[i], values[i]))
}

fn property_value(card: &Card, object: impl ResourceHandle, name: &str) -> Option<u64> {
    property(card, object, name).map(|p| p.1)
}

fn gamma_blob(card: &Card, size: Option<u32>, ramps: Option<&[u16]>) -> Result<u32> {
    let (Some(ramps), Some(size)) = (ramps, size) else { return Ok(0) };
    let size = size as usize;
    if ramps.len() != size * 3 {
        return Err("gamma ramps do not match the lut size".into());
    }
    let mut lut: Vec<u8> = (0..size)
        .flat_map(|i| [ramps[i], ramps[size + i], ramps[2 * size + i], 0])
        .flat_map(u16::to_ne_bytes)
        .collect();
    Ok(drm_ffi::mode::create_property_blob(card.as_fd(), &mut lut)?.blob_id)
}

fn flip_async(card: &Card, o: &Output, framebuffer: framebuffer::Handle, fence: Option<property::Value>) -> Result<()> {
    let mut request = Request::new(o);
    if !request.set(o.plane, "FB_ID", property::Value::Framebuffer(Some(framebuffer))) {
        return Err("no FB_ID".into());
    }
    if let Some(fence) = fence {
        request.set(o.plane, "IN_FENCE_FD", fence);
    }
    let flags = AtomicCommitFlags::PAGE_FLIP_EVENT | AtomicCommitFlags::NONBLOCK | AtomicCommitFlags::PAGE_FLIP_ASYNC;
    card.atomic_commit(flags, request.0)?;
    Ok(())
}

struct Request<'a>(AtomicModeReq, &'a HashMap<(u32, &'static str), property::Handle>);

impl<'a> Request<'a> {
    fn new(o: &'a Output) -> Self {
        Self(AtomicModeReq::new(), &o.properties)
    }

    fn set(&mut self, object: impl Into<u32>, name: &'static str, value: property::Value) -> bool {
        let object = object.into();
        let Some(&property) = self.1.get(&(object, name)) else { return false };
        let handle = drm::control::RawResourceHandle::new(object).expect("objects are non-zero");
        self.0.add_raw_property(handle, property, value.into());
        true
    }

    fn modeset(&mut self, o: &Output, blob: u64, vrr: bool) {
        self.set(o.connector, "CRTC_ID", property::Value::CRTC(Some(o.crtc)));
        self.set(o.crtc, "MODE_ID", property::Value::Blob(blob));
        self.set(o.crtc, "ACTIVE", property::Value::Boolean(true));
        self.set(o.crtc, "VRR_ENABLED", property::Value::UnsignedRange(vrr.into()));
    }

    fn place(
        &mut self,
        plane: plane::Handle,
        source: Option<(framebuffer::Handle, crtc::Handle)>,
        (x, y): (i32, i32),
        (w, h): (u32, u32),
    ) {
        self.set(plane, "FB_ID", property::Value::Framebuffer(source.map(|s| s.0)));
        self.set(plane, "CRTC_ID", property::Value::CRTC(source.map(|s| s.1)));
        if source.is_none() {
            return;
        }
        let (w, h) = (u64::from(w), u64::from(h));
        for (name, value) in
            [("SRC_X", 0), ("SRC_Y", 0), ("SRC_W", w << 16), ("SRC_H", h << 16), ("CRTC_W", w), ("CRTC_H", h)]
        {
            self.set(plane, name, property::Value::UnsignedRange(value));
        }
        self.set(plane, "CRTC_X", property::Value::SignedRange(x.into()));
        self.set(plane, "CRTC_Y", property::Value::SignedRange(y.into()));
    }
}

fn mode_blob(card: &Card, mode: &DrmMode) -> Result<u64> {
    match card.create_property_blob(mode)? {
        property::Value::Blob(blob) => Ok(blob),
        _ => Err("no mode blob".into()),
    }
}

fn test_mode(card: &Card, o: &Output, chosen: DrmMode, vrr: bool, framebuffer: framebuffer::Handle) -> Result<()> {
    let blob = mode_blob(card, &chosen)?;
    let mut request = Request::new(o);
    request.modeset(o, blob, vrr);
    let (w, h) = chosen.size();
    request.place(o.plane, Some((framebuffer, o.crtc)), (0, 0), (w.into(), h.into()));
    let flags = AtomicCommitFlags::TEST_ONLY | AtomicCommitFlags::ALLOW_MODESET;
    let result = card.atomic_commit(flags, request.0);
    let _ = card.destroy_property_blob(blob);
    Ok(result?)
}

fn destroy_buffers(card: &Card, o: &mut Output) {
    for buffer in o.buffers.drain(..) {
        let _ = card.destroy_framebuffer(buffer.framebuffer);
    }
}

fn release(card: &Card, mut o: Output) {
    destroy_buffers(card, &mut o);
    for blob in [o.mode_blob, o.gamma_blob.into()].into_iter().filter(|&b| b != 0) {
        let _ = card.destroy_property_blob(blob);
    }
    o.cursor.into_iter().for_each(|c| c.destroy(card));
}

fn allocate(
    card: &Card,
    gbm: &gbm::Device<Card>,
    output: &Output,
    mode: DrmMode,
    formats: &[(u32, u64)],
) -> Result<Vec<Scanout>> {
    let mut modifiers: Vec<u64> =
        formats.iter().filter(|f| f.0 == XRGB8888 && output.formats.contains(f)).map(|f| f.1).collect();
    if modifiers.is_empty() {
        modifiers.push(0);
    }
    let (w, h) = mode.size();
    let usage = BufferObjectFlags::SCANOUT | BufferObjectFlags::RENDERING;
    (0..BUFFERS)
        .map(|_| {
            let mods = modifiers.iter().map(|&m| Modifier::from(m));
            let bo =
                gbm.create_buffer_object_with_modifiers2::<()>(w.into(), h.into(), gbm::Format::Xrgb8888, mods, usage)?;
            if bo.plane_count() != 1 {
                return Err("multi-plane scanout buffers are not supported".into());
            }
            let framebuffer = card.add_planar_framebuffer(&bo, FbCmd2Flags::MODIFIERS)?;
            let plane = gfx::Plane { fd: bo.fd()?, offset: bo.offset(0), stride: bo.stride() };
            let modifier = u64::from(bo.modifier());
            let dmabuf = gfx::Dmabuf { size: (w.into(), h.into()), format: XRGB8888, modifier, planes: vec![plane] };
            Ok(Scanout { dmabuf, serial: 0, framebuffer, _bo: bo })
        })
        .collect()
}

impl Drop for Kms {
    fn drop(&mut self) {
        for o in self.outputs.drain(..) {
            release(&self.card, o);
        }
        for (_, fb) in self.clients.drain() {
            planes::release(&self.card, fb, &mut self.handles);
        }
        let mut session = self.session.borrow_mut();
        let devices: Vec<_> = session.devices.drain().map(|(_, d)| d).collect();
        if let Some(seat) = session.seat.as_mut() {
            for device in devices {
                let _ = seat.close_device(device);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn edid_identity() {
        let mut edid = vec![0u8; 128];
        edid[8..10].copy_from_slice(&0x0472u16.to_be_bytes());
        edid[12..16].copy_from_slice(&0x1234u32.to_le_bytes());
        edid[57] = 0xfc;
        edid[59..72].copy_from_slice(b"VG270U P\n    ");
        assert_eq!(super::edid_identity(&edid), ["ACR".to_string(), "VG270U P".into(), "0x00001234".into()]);
        edid[75] = 0xff;
        edid[77..90].copy_from_slice(b"ABC123\n      ");
        assert_eq!(super::edid_identity(&edid)[2], "ABC123");
    }
}
