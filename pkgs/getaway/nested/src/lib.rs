use backend::{Event, InputEvent, Mode, OutputEvent, OutputId, OutputInfo, Scroll, ScrollAxis};
use rustix::{
    fs::{MemfdFlags, ftruncate, memfd_create},
    mm::{MapFlags, ProtFlags, mmap, munmap},
};
use std::{
    error::Error,
    fs::{File, OpenOptions},
    io::ErrorKind,
    os::fd::{AsFd, BorrowedFd},
    ptr,
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering::Relaxed},
    },
};
use wayland_client::{
    Connection, Dispatch, EventQueue, QueueHandle, WEnum,
    backend::WaylandError,
    delegate_noop,
    globals::{GlobalListContents, registry_queue_init},
    protocol::{
        wl_buffer::{self, WlBuffer},
        wl_callback::{self, WlCallback},
        wl_compositor::WlCompositor,
        wl_keyboard::{self, WlKeyboard},
        wl_pointer::{self, WlPointer},
        wl_registry::WlRegistry,
        wl_seat::{self, WlSeat},
        wl_shm::{self, WlShm},
        wl_shm_pool::WlShmPool,
        wl_surface::{self, WlSurface},
    },
};
use wayland_protocols::wp::{
    fractional_scale::v1::client::{
        wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
        wp_fractional_scale_v1::{self, WpFractionalScaleV1},
    },
    linux_dmabuf::zv1::client::{
        zwp_linux_buffer_params_v1::{Flags, ZwpLinuxBufferParamsV1},
        zwp_linux_dmabuf_v1::{self, ZwpLinuxDmabufV1},
    },
    pointer_constraints::zv1::client::{
        zwp_confined_pointer_v1::ZwpConfinedPointerV1,
        zwp_locked_pointer_v1::ZwpLockedPointerV1,
        zwp_pointer_constraints_v1::{Lifetime, ZwpPointerConstraintsV1},
    },
    relative_pointer::zv1::client::{
        zwp_relative_pointer_manager_v1::ZwpRelativePointerManagerV1,
        zwp_relative_pointer_v1::{self, ZwpRelativePointerV1},
    },
    viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter},
};
use wayland_protocols::xdg::shell::client::{
    xdg_surface::{self, XdgSurface},
    xdg_toplevel::{self, XdgToplevel},
    xdg_wm_base::{self, XdgWmBase},
};

pub type Result<T> = std::result::Result<T, Box<dyn Error>>;

enum Constraint {
    Lock(ZwpLockedPointerV1),
    Confine(ZwpConfinedPointerV1),
}

pub struct Nested {
    conn: Connection,
    queue: EventQueue<Host>,
    host: Host,
}

struct Host {
    qh: QueueHandle<Host>,
    shm: WlShm,
    windows: Vec<Window>,
    keyboard: Option<WlKeyboard>,
    keys: Vec<u32>,
    pointer: Option<WlPointer>,
    pointer_window: Option<OutputId>,
    relative: Option<ZwpRelativePointerManagerV1>,
    constraints: Option<ZwpPointerConstraintsV1>,
    constraint: Option<Constraint>,
    events: Vec<Event>,
    scroll: Option<Scroll>,
    host_formats: Vec<(u32, u64)>,
    dmabuf: Option<Dmabufs>,
}

struct Dmabufs {
    manager: ZwpLinuxDmabufV1,
    gbm: gbm::Device<File>,
    modifiers: Vec<u64>,
    next_serial: u64,
}

const XRGB8888: u32 = 0x3432_5258;

pub enum Target<'a> {
    Memory(&'a mut [u32]),
    Dmabuf(u64, &'a gfx::Dmabuf),
}

struct Window {
    id: OutputId,
    surface: WlSurface,
    _toplevel: XdgToplevel,
    viewport: Option<WpViewport>,
    fractional: bool,
    size: (i32, i32),
    scale: f64,
    configured: bool,
    frame_pending: bool,
    buffers: Vec<Buffer>,
}

impl Window {
    fn pixels(&self) -> (i32, i32) {
        let f = |v: i32| (f64::from(v) * self.scale).round() as i32;
        (f(self.size.0), f(self.size.1))
    }

    fn info(&self) -> OutputInfo {
        let (width, height) = self.pixels();
        let mode = Mode { width, height, refresh: 60_000 };
        let name = format!("NESTED-{}", self.id);
        let (make, model, modes, preferred) = ("getaway".into(), "nested".into(), vec![mode], Some(mode));
        OutputInfo { id: self.id, name, make, model, serial: String::new(), modes, preferred, mode, scale: self.scale }
    }
}

struct Buffer {
    wl: WlBuffer,
    pixels: *mut u32,
    len: usize,
    size: (i32, i32),
    busy: Arc<AtomicBool>,
    dmabuf: Option<(u64, gfx::Dmabuf, gbm::BufferObject<()>)>,
}

impl Buffer {
    fn new(host: &mut Host, size: (i32, i32)) -> Result<Self> {
        match host.dmabuf.as_mut() {
            Some(d) => Self::dmabuf(d, &host.qh, size),
            None => Self::shm(host, size),
        }
    }

    fn dmabuf(d: &mut Dmabufs, qh: &QueueHandle<Host>, (w, h): (i32, i32)) -> Result<Self> {
        let modifiers = d.modifiers.iter().map(|&m| gbm::Modifier::from(m));
        let flags = gbm::BufferObjectFlags::RENDERING;
        let bo = d.gbm.create_buffer_object_with_modifiers2::<()>(
            w as u32,
            h as u32,
            gbm::Format::Xrgb8888,
            modifiers,
            flags,
        )?;
        if bo.plane_count() != 1 {
            return Err("multi-plane buffers are not supported".into());
        }
        let (fd, offset, stride, modifier) = (bo.fd()?, bo.offset(0), bo.stride(), u64::from(bo.modifier()));
        let params = d.manager.create_params(qh, ());
        params.add(fd.as_fd(), 0, offset, stride, (modifier >> 32) as u32, modifier as u32);
        let busy = Arc::new(AtomicBool::new(false));
        let wl = params.create_immed(w, h, XRGB8888, Flags::empty(), qh, busy.clone());
        params.destroy();
        d.next_serial += 1;
        let planes = vec![gfx::Plane { fd, offset, stride }];
        let dmabuf = gfx::Dmabuf { size: (w, h), format: XRGB8888, modifier, planes };
        let (pixels, len, dmabuf) = (ptr::null_mut(), 0, Some((d.next_serial, dmabuf, bo)));
        Ok(Self { wl, pixels, len, size: (w, h), busy, dmabuf })
    }

    fn shm(host: &Host, (w, h): (i32, i32)) -> Result<Self> {
        let len = (w * h * 4) as usize;
        let fd = memfd_create("getaway-nested", MemfdFlags::CLOEXEC)?;
        ftruncate(&fd, len as u64)?;
        let pixels =
            unsafe { mmap(ptr::null_mut(), len, ProtFlags::READ | ProtFlags::WRITE, MapFlags::SHARED, &fd, 0)? };
        let pool = host.shm.create_pool(fd.as_fd(), len as i32, &host.qh, ());
        let busy = Arc::new(AtomicBool::new(false));
        let wl = pool.create_buffer(0, w, h, w * 4, wl_shm::Format::Xrgb8888, &host.qh, busy.clone());
        pool.destroy();
        Ok(Self { wl, pixels: pixels.cast(), len, size: (w, h), busy, dmabuf: None })
    }

    fn pixels(&mut self) -> &mut [u32] {
        unsafe { std::slice::from_raw_parts_mut(self.pixels, self.len / 4) }
    }
}

impl Drop for Buffer {
    fn drop(&mut self) {
        self.wl.destroy();
        if self.len > 0 {
            let _ = unsafe { munmap(self.pixels.cast(), self.len) };
        }
    }
}

impl Nested {
    pub fn connect(outputs: u32, render: (u64, &[(u32, u64)])) -> Result<Self> {
        let conn = Connection::connect_to_env()?;
        let (globals, mut queue) = registry_queue_init::<Host>(&conn)?;
        let qh = queue.handle();
        let compositor: WlCompositor = globals.bind(&qh, 4..=6, ())?;
        let wm_base: XdgWmBase = globals.bind(&qh, 1..=6, ())?;
        globals.bind::<WlSeat, _, _>(&qh, 1..=9, ())?;
        let viewporter = globals.bind::<WpViewporter, _, _>(&qh, 1..=1, ()).ok();
        let fractional = globals.bind::<WpFractionalScaleManagerV1, _, _>(&qh, 1..=1, ()).ok();
        let windows = (1..=outputs.max(1))
            .map(|id| {
                let surface = compositor.create_surface(&qh, id);
                let toplevel = wm_base.get_xdg_surface(&surface, &qh, id).get_toplevel(&qh, id);
                toplevel.set_title(format!("getaway {id}"));
                toplevel.set_app_id("getaway".into());
                let viewport = viewporter.as_ref().map(|v| v.get_viewport(&surface, &qh, ()));
                let fractional = fractional.as_ref().filter(|_| viewport.is_some());
                fractional.iter().for_each(|f| drop(f.get_fractional_scale(&surface, &qh, id)));
                surface.commit();
                let (fractional, size, scale) = (fractional.is_some(), (1280, 800), 1.0);
                let (configured, frame_pending, buffers) = (false, false, vec![]);
                Window {
                    id,
                    surface,
                    _toplevel: toplevel,
                    viewport,
                    fractional,
                    size,
                    scale,
                    configured,
                    frame_pending,
                    buffers,
                }
            })
            .collect();
        let mut host = Host {
            shm: globals.bind(&qh, 1..=1, ())?,
            relative: globals.bind(&qh, 1..=1, ()).ok(),
            constraints: globals.bind(&qh, 1..=1, ()).ok(),
            qh,
            windows,
            keyboard: None,
            keys: vec![],
            pointer: None,
            pointer_window: None,
            constraint: None,
            events: vec![],
            scroll: None,
            host_formats: vec![],
            dmabuf: None,
        };
        let manager = globals.bind::<ZwpLinuxDmabufV1, _, _>(&host.qh, 3..=3, ()).ok();
        queue.roundtrip(&mut host)?;
        host.dmabuf = manager.and_then(|manager| Dmabufs::new(manager, &host.host_formats, render));
        log::info!("nested presentation via {}", if host.dmabuf.is_some() { "dmabuf" } else { "shm" });
        while host.windows.iter().any(|w| !w.configured) {
            queue.blocking_dispatch(&mut host)?;
        }
        host.events.retain(|e| !matches!(e, Event::Output(OutputEvent::Changed(_))));
        let added: Vec<_> = host.windows.iter().map(|w| Event::Output(OutputEvent::Added(w.info()))).collect();
        host.events.splice(0..0, added);
        Ok(Self { conn, queue, host })
    }

    pub fn fd(&self) -> BorrowedFd<'_> {
        self.conn.as_fd()
    }

    pub fn ready(&self, output: OutputId) -> bool {
        self.host.window(output).is_some_and(|w| !w.frame_pending)
    }

    pub fn dispatch(&mut self, readable: bool, events: &mut Vec<Event>) -> Result<()> {
        if readable && let Some(guard) = self.queue.prepare_read() {
            ignore_would_block(guard.read().map(drop))?;
        }
        self.queue.dispatch_pending(&mut self.host)?;
        events.append(&mut self.host.events);
        ignore_would_block(self.conn.flush())
    }

    pub fn constrain(&mut self, lock: Option<bool>) {
        let host = &mut self.host;
        match host.constraint.take() {
            Some(Constraint::Lock(c)) => c.destroy(),
            Some(Constraint::Confine(c)) => c.destroy(),
            None => {}
        }
        let surface = host.pointer_window.and_then(|id| host.window(id)).map(|w| w.surface.clone());
        let (Some(lock), Some(manager), Some(pointer), Some(surface)) =
            (lock, &host.constraints, &host.pointer, surface)
        else {
            return;
        };
        let qh = &host.qh;
        host.constraint = Some(match lock {
            true => Constraint::Lock(manager.lock_pointer(&surface, pointer, None, Lifetime::Persistent, qh, ())),
            false => {
                Constraint::Confine(manager.confine_pointer(&surface, pointer, None, Lifetime::Persistent, qh, ()))
            }
        });
        let _ = self.conn.flush();
    }

    pub fn serials(&self) -> Vec<u64> {
        let buffers = self.host.windows.iter().flat_map(|w| &w.buffers);
        buffers.filter_map(|b| b.dmabuf.as_ref().map(|d| d.0)).collect()
    }

    pub fn present(&mut self, output: OutputId, draw: impl FnOnce(Target)) -> Result<bool> {
        let host = &mut self.host;
        let Some(index) = host.windows.iter().position(|w| w.id == output) else { return Ok(false) };
        let size = host.windows[index].pixels();
        host.windows[index].buffers.retain(|b| b.size == size);
        let free = host.windows[index].buffers.iter().position(|b| !b.busy.load(Relaxed));
        let slot = match free {
            Some(i) => i,
            None if host.windows[index].buffers.len() < 3 => {
                let buffer = Buffer::new(host, size)?;
                host.windows[index].buffers.push(buffer);
                host.windows[index].buffers.len() - 1
            }
            None => return Ok(false),
        };
        let qh = host.qh.clone();
        let window = &mut host.windows[index];
        let buffer = &mut window.buffers[slot];
        match &buffer.dmabuf {
            Some((serial, dmabuf, _)) => draw(Target::Dmabuf(*serial, dmabuf)),
            None => draw(Target::Memory(buffer.pixels())),
        }
        buffer.busy.store(true, Relaxed);
        window.surface.attach(Some(&buffer.wl), 0, 0);
        window.surface.damage_buffer(0, 0, size.0, size.1);
        match &window.viewport {
            Some(viewport) => viewport.set_destination(window.size.0, window.size.1),
            None => window.surface.set_buffer_scale(window.scale as i32),
        }
        window.surface.frame(&qh, window.id);
        window.surface.commit();
        window.frame_pending = true;
        ignore_would_block(self.conn.flush())?;
        Ok(true)
    }
}

impl Host {
    fn window(&self, id: OutputId) -> Option<&Window> {
        self.windows.iter().find(|w| w.id == id)
    }

    fn window_mut(&mut self, id: OutputId) -> Option<&mut Window> {
        self.windows.iter_mut().find(|w| w.id == id)
    }

    fn update(&mut self, id: OutputId, f: impl FnOnce(&mut Window) -> bool) {
        let Some(window) = self.window_mut(id) else { return };
        if f(window) && window.configured {
            let info = window.info();
            self.events.push(Event::Output(OutputEvent::Changed(info)));
        }
    }
}

fn ignore_would_block(r: std::result::Result<(), WaylandError>) -> Result<()> {
    match r {
        Err(WaylandError::Io(e)) if e.kind() == ErrorKind::WouldBlock => Ok(()),
        r => Ok(r?),
    }
}

fn pressed<T: PartialEq>(state: WEnum<T>, value: T) -> bool {
    state == WEnum::Value(value)
}

fn changed<T: PartialEq>(slot: &mut T, value: T) -> bool {
    *slot != value && {
        *slot = value;
        true
    }
}

delegate_noop!(Host: WlCompositor);
delegate_noop!(Host: WlShmPool);
delegate_noop!(Host: ZwpLinuxBufferParamsV1);
delegate_noop!(Host: ignore WlShm);
delegate_noop!(Host: WpViewporter);
delegate_noop!(Host: WpViewport);
delegate_noop!(Host: WpFractionalScaleManagerV1);
delegate_noop!(Host: ZwpRelativePointerManagerV1);
delegate_noop!(Host: ZwpPointerConstraintsV1);
delegate_noop!(Host: ignore ZwpLockedPointerV1);
delegate_noop!(Host: ignore ZwpConfinedPointerV1);

impl Dispatch<WlRegistry, GlobalListContents> for Host {
    fn event(
        _: &mut Self,
        _: &WlRegistry,
        _: <WlRegistry as wayland_client::Proxy>::Event,
        _: &GlobalListContents,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
    }
}

impl Dispatch<WlSurface, OutputId> for Host {
    fn event(
        host: &mut Self,
        _: &WlSurface,
        event: wl_surface::Event,
        &id: &OutputId,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_surface::Event::PreferredBufferScale { factor } = event {
            host.update(id, |w| !w.fractional && changed(&mut w.scale, f64::from(factor.max(1))));
        }
    }
}

impl Dispatch<WpFractionalScaleV1, OutputId> for Host {
    fn event(
        host: &mut Self,
        _: &WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        &id: &OutputId,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wp_fractional_scale_v1::Event::PreferredScale { scale } = event {
            host.update(id, |w| changed(&mut w.scale, f64::from(scale) / 120.0));
        }
    }
}

impl Dispatch<XdgWmBase, ()> for Host {
    fn event(_: &mut Self, base: &XdgWmBase, event: xdg_wm_base::Event, _: &(), _: &Connection, _: &QueueHandle<Self>) {
        if let xdg_wm_base::Event::Ping { serial } = event {
            base.pong(serial);
        }
    }
}

impl Dispatch<XdgSurface, OutputId> for Host {
    fn event(
        host: &mut Self,
        surface: &XdgSurface,
        event: xdg_surface::Event,
        &id: &OutputId,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let xdg_surface::Event::Configure { serial } = event {
            surface.ack_configure(serial);
            if let Some(window) = host.window_mut(id) {
                window.configured = true;
            }
        }
    }
}

impl Dispatch<XdgToplevel, OutputId> for Host {
    fn event(
        host: &mut Self,
        _: &XdgToplevel,
        event: xdg_toplevel::Event,
        &id: &OutputId,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        match event {
            xdg_toplevel::Event::Configure { width, height, .. } if width > 0 && height > 0 => {
                host.update(id, |w| changed(&mut w.size, (width, height)));
            }
            xdg_toplevel::Event::Close => host.events.push(Event::Close),
            _ => {}
        }
    }
}

impl Dispatch<WlCallback, OutputId> for Host {
    fn event(
        host: &mut Self,
        _: &WlCallback,
        event: wl_callback::Event,
        &id: &OutputId,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_callback::Event::Done { .. } = event
            && let Some(window) = host.window_mut(id)
        {
            window.frame_pending = false;
            host.events.push(Event::Frame { output: id, flip: None });
        }
    }
}

impl Dmabufs {
    fn new(manager: ZwpLinuxDmabufV1, host: &[(u32, u64)], (render_node, ours): (u64, &[(u32, u64)])) -> Option<Self> {
        let path = format!("/dev/dri/renderD{}", rustix::fs::minor(render_node));
        let gbm = gbm::Device::new(OpenOptions::new().read(true).write(true).open(path).ok()?).ok()?;
        let modifiers: Vec<u64> = host.iter().filter(|f| f.0 == XRGB8888 && ours.contains(f)).map(|f| f.1).collect();
        (!modifiers.is_empty()).then_some(Self { manager, gbm, modifiers, next_serial: 0 })
    }
}

impl Dispatch<ZwpLinuxDmabufV1, ()> for Host {
    fn event(
        host: &mut Self,
        _: &ZwpLinuxDmabufV1,
        event: zwp_linux_dmabuf_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwp_linux_dmabuf_v1::Event::Modifier { format, modifier_hi, modifier_lo } = event {
            host.host_formats.push((format, (u64::from(modifier_hi) << 32) | u64::from(modifier_lo)));
        }
    }
}

impl Dispatch<WlBuffer, Arc<AtomicBool>> for Host {
    fn event(
        _: &mut Self,
        _: &WlBuffer,
        event: wl_buffer::Event,
        busy: &Arc<AtomicBool>,
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let wl_buffer::Event::Release = event {
            busy.store(false, Relaxed);
        }
    }
}

impl Dispatch<WlSeat, ()> for Host {
    fn event(host: &mut Self, seat: &WlSeat, event: wl_seat::Event, _: &(), _: &Connection, qh: &QueueHandle<Self>) {
        let wl_seat::Event::Capabilities { capabilities: WEnum::Value(caps) } = event else { return };
        match (caps.contains(wl_seat::Capability::Keyboard), host.keyboard.take()) {
            (true, None) => host.keyboard = Some(seat.get_keyboard(qh, ())),
            (false, Some(keyboard)) => keyboard.release(),
            (_, keyboard) => host.keyboard = keyboard,
        }
        match (caps.contains(wl_seat::Capability::Pointer), host.pointer.take()) {
            (true, None) => {
                let pointer = seat.get_pointer(qh, ());
                if let Some(relative) = &host.relative {
                    relative.get_relative_pointer(&pointer, qh, ());
                }
                host.pointer = Some(pointer);
            }
            (false, Some(pointer)) => pointer.release(),
            (_, pointer) => host.pointer = pointer,
        }
    }
}

impl Dispatch<WlKeyboard, ()> for Host {
    fn event(
        host: &mut Self,
        _: &WlKeyboard,
        event: wl_keyboard::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        host.events.push(Event::Input(match event {
            wl_keyboard::Event::Keymap { fd, size, .. } => InputEvent::Keymap(fd, size),
            wl_keyboard::Event::Key { key, state, .. } => {
                let pressed = pressed(state, wl_keyboard::KeyState::Pressed);
                host.keys.retain(|&k| k != key);
                if pressed {
                    host.keys.push(key);
                }
                InputEvent::Key { key, pressed }
            }
            wl_keyboard::Event::Leave { .. } => {
                let released = host.keys.drain(..).map(|key| InputEvent::Key { key, pressed: false });
                host.events.extend(released.map(Event::Input));
                return;
            }
            wl_keyboard::Event::Modifiers { mods_depressed, mods_latched, mods_locked, group, .. } => {
                InputEvent::Modifiers([mods_depressed, mods_latched, mods_locked, group])
            }
            _ => return,
        }));
    }
}

impl Dispatch<WlPointer, ()> for Host {
    fn event(
        host: &mut Self,
        pointer: &WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        let motion =
            |host: &Host, x: f64, y: f64| host.pointer_window.map(|output| InputEvent::Motion { output, x, y });
        let event = match event {
            wl_pointer::Event::Enter { serial, surface, surface_x, surface_y } => {
                pointer.set_cursor(serial, None, 0, 0);
                host.pointer_window = host.windows.iter().find(|w| w.surface == surface).map(|w| w.id);
                motion(host, surface_x, surface_y)
            }
            wl_pointer::Event::Motion { surface_x, surface_y, .. } => motion(host, surface_x, surface_y),
            wl_pointer::Event::Leave { .. } => Some(InputEvent::PointerLeft),
            wl_pointer::Event::Button { button, state, .. } => {
                Some(InputEvent::Button { button, pressed: pressed(state, wl_pointer::ButtonState::Pressed) })
            }
            wl_pointer::Event::AxisSource { axis_source: WEnum::Value(source) } => {
                host.scroll.get_or_insert_default().source = source.into();
                None
            }
            wl_pointer::Event::Axis { axis: WEnum::Value(axis), value, .. } => {
                host.scroll_axis(axis, |a| a.value = value);
                None
            }
            wl_pointer::Event::AxisStop { axis: WEnum::Value(axis), .. } => {
                host.scroll_axis(axis, |a| a.value = 0.0);
                None
            }
            wl_pointer::Event::AxisValue120 { axis: WEnum::Value(axis), value120 } => {
                host.scroll_axis(axis, |a| a.value120 = value120);
                None
            }
            wl_pointer::Event::AxisRelativeDirection { axis: WEnum::Value(axis), direction } => {
                let inverted = direction == WEnum::Value(wl_pointer::AxisRelativeDirection::Inverted);
                host.scroll_axis(axis, |a| a.inverted = inverted);
                None
            }
            wl_pointer::Event::Frame => host.scroll.take().map(InputEvent::Scroll),
            _ => None,
        };
        host.events.extend(event.map(Event::Input));
    }
}

impl Host {
    fn scroll_axis(&mut self, axis: wl_pointer::Axis, f: impl FnOnce(&mut ScrollAxis)) {
        let scroll = self.scroll.get_or_insert_default();
        let index = if axis == wl_pointer::Axis::HorizontalScroll { 1 } else { 0 };
        f(scroll.axes[index].get_or_insert_default());
    }
}

impl Dispatch<ZwpRelativePointerV1, ()> for Host {
    fn event(
        host: &mut Self,
        _: &ZwpRelativePointerV1,
        event: zwp_relative_pointer_v1::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<Self>,
    ) {
        if let zwp_relative_pointer_v1::Event::RelativeMotion { utime_hi, utime_lo, dx, dy, dx_unaccel, dy_unaccel } =
            event
        {
            let usec = (u64::from(utime_hi) << 32) | u64::from(utime_lo);
            host.events
                .push(Event::Input(InputEvent::RelativeMotion { delta: [dx, dy, dx_unaccel, dy_unaccel], usec }));
        }
    }
}
