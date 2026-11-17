use crate::{
    Caps, ClientId, State,
    buffer::{self, Buffer},
    caps,
    client::Data,
    output,
    shm::{Pool, ShmBuffer},
    surface::Surface,
};
use std::{os::fd::OwnedFd, rc::Rc};
use wire::*;
use wm::Rect;

const GLOBALS: &[(Interface, u32, Caps)] = &[
    (Interface::WlCompositor, 7, caps::NONE),
    (Interface::WlFixes, 2, caps::NONE),
    (Interface::WpSinglePixelBufferManagerV1, 1, caps::NONE),
    (Interface::WpAlphaModifierV1, 1, caps::NONE),
    (Interface::WpContentTypeManagerV1, 1, caps::NONE),
    (Interface::WpPointerWarpV1, 1, caps::NONE),
    (Interface::WpLinuxDrmSyncobjManagerV1, 1, caps::NONE),
    (Interface::WpFifoManagerV1, 1, caps::NONE),
    (Interface::WpCommitTimingManagerV1, 1, caps::NONE),
    (Interface::WpTearingControlManagerV1, 1, caps::NONE),
    (Interface::XdgWmDialogV1, 1, caps::NONE),
    (Interface::XdgToplevelIconManagerV1, 1, caps::NONE),
    (Interface::XdgToplevelTagManagerV1, 1, caps::NONE),
    (Interface::XdgSystemBellV1, 1, caps::NONE),
    (Interface::OrgKdeKwinServerDecorationManager, 1, caps::NONE),
    (Interface::WlSubcompositor, 1, caps::NONE),
    (Interface::WlShm, 3, caps::NONE),
    (Interface::ZwpLinuxDmabufV1, 6, caps::NONE),
    (Interface::WlSeat, 11, caps::NONE),
    (Interface::WlDataDeviceManager, 4, caps::NONE),
    (Interface::ZwpPrimarySelectionDeviceManagerV1, 1, caps::NONE),
    (Interface::XdgWmBase, 7, caps::NONE),
    (Interface::ZxdgDecorationManagerV1, 2, caps::NONE),
    (Interface::XdgToplevelDragManagerV1, 1, caps::NONE),
    (Interface::WpViewporter, 1, caps::NONE),
    (Interface::WpFractionalScaleManagerV1, 1, caps::NONE),
    (Interface::ZxdgOutputManagerV1, 3, caps::NONE),
    (Interface::WpPresentation, 3, caps::NONE),
    (Interface::ZwpIdleInhibitManagerV1, 1, caps::NONE),
    (Interface::XdgActivationV1, 1, caps::NONE),
    (Interface::WpCursorShapeManagerV1, 2, caps::NONE),
    (Interface::ZwpPointerConstraintsV1, 1, caps::NONE),
    (Interface::ZwpRelativePointerManagerV1, 1, caps::NONE),
    (Interface::ZwpPointerGesturesV1, 3, caps::NONE),
    (Interface::ZwlrLayerShellV1, 4, caps::LAYER_SHELL),
    (Interface::ZwlrOutputManagerV1, 4, caps::OUTPUT_MANAGEMENT),
    (Interface::ZwlrGammaControlManagerV1, 1, caps::GAMMA),
    (Interface::ZwlrOutputPowerManagerV1, 1, caps::OUTPUT_POWER),
    (Interface::ExtImageCopyCaptureManagerV1, 1, caps::SCREENCOPY),
    (Interface::ExtOutputImageCaptureSourceManagerV1, 1, caps::SCREENCOPY),
    (Interface::ExtForeignToplevelImageCaptureSourceManagerV1, 1, caps::SCREENCOPY),
    (Interface::ZwpVirtualKeyboardManagerV1, 1, caps::VIRTUAL_INPUT),
    (Interface::ZwlrVirtualPointerManagerV1, 2, caps::VIRTUAL_INPUT),
    (Interface::ExtSessionLockManagerV1, 1, caps::SESSION_LOCK),
    (Interface::ExtIdleNotifierV1, 2, caps::IDLE_NOTIFY),
    (Interface::ZwlrForeignToplevelManagerV1, 3, caps::FOREIGN_TOPLEVEL),
    (Interface::ExtForeignToplevelListV1, 1, caps::FOREIGN_TOPLEVEL),
    (Interface::ExtWorkspaceManagerV1, 1, caps::WORKSPACES),
    (Interface::ExtDataControlManagerV1, 1, caps::DATA_CONTROL),
    (Interface::ZwlrDataControlManagerV1, 2, caps::DATA_CONTROL),
];

impl wl_display::Handler for State {
    fn sync(&mut self, cx: Cx, callback: Obj) {
        callback_done(self, cx.client, callback.id, self.serial);
    }

    fn get_registry(&mut self, cx: Cx, registry: Obj) {
        let outputs = output::advertised(self);
        let client = self.client(cx.client);
        for (name, &(interface, version, needs)) in (1..).zip(GLOBALS) {
            if client.caps & needs == needs {
                wl_registry::global(&mut client.conn.out, registry, name, interface.name(), version);
            }
        }
        for name in outputs {
            wl_registry::global(&mut client.conn.out, registry, name, wl_output::NAME, output::VERSION);
        }
    }
}

pub fn callback_done(st: &mut State, c: ClientId, id: u32, data: u32) {
    crate::client::send_destructor(st, c, Obj { id, version: 1 }, |out, obj| wl_callback::done(out, obj, data));
}

impl wl_registry::Handler for State {
    fn bind(&mut self, cx: Cx, name: u32, id: NewId) -> Res {
        let c = cx.client;
        if name >= output::GLOBAL_BASE && id.interface == wl_output::NAME {
            return output::bind(self, c, name, id.id, id.version);
        }
        let allowed = self.client(c).caps;
        let global = GLOBALS.get(name.wrapping_sub(1) as usize);
        let global = global.filter(|g| g.0.name() == id.interface && allowed & g.2 == g.2);
        let Some(&(interface, ..)) = global.filter(|g| (1..=g.1).contains(&id.version)) else {
            let message = format!("cannot bind {} version {} as global {name}", id.interface, id.version);
            return Err(cx.error(wl_display::error::INVALID_OBJECT, message));
        };
        let bound = self.client(c).add(id.id, interface, id.version, 0)?;
        match interface {
            Interface::WlShm => self.send(c, |out| {
                wl_shm::format(out, bound, wl_shm::format::ARGB8888);
                wl_shm::format(out, bound, wl_shm::format::XRGB8888);
            }),
            Interface::WlSeat => self.send(c, |out| {
                wl_seat::capabilities(out, bound, wl_seat::capability::POINTER | wl_seat::capability::KEYBOARD);
                wl_seat::name(out, bound, "seat0");
            }),
            Interface::ZwpLinuxDmabufV1 => crate::dmabuf::bound(self, c, bound),
            Interface::WpPresentation => {
                self.send(c, |out| wp_presentation::clock_id(out, bound, libc::CLOCK_MONOTONIC as u32))
            }
            Interface::ZwlrForeignToplevelManagerV1 | Interface::ExtForeignToplevelListV1 => {
                self.set((c, id.id), Data::Toplevels(vec![]));
                crate::scale::resync(self);
            }
            Interface::ExtWorkspaceManagerV1 => {
                self.set((c, id.id), Data::Workspaces(Default::default()));
                crate::scale::resync(self);
            }
            Interface::ZwlrOutputManagerV1 => crate::privileged::output_management::bound(self, c, bound),
            Interface::XdgToplevelIconManagerV1 => self.send(c, |out| xdg_toplevel_icon_manager_v1::done(out, bound)),
            Interface::OrgKdeKwinServerDecorationManager => {
                self.send(c, |out| org_kde_kwin_server_decoration_manager::default_mode(out, bound, KDE_SERVER_SIDE))
            }
            _ => {}
        }
        Ok(())
    }
}

impl wp_presentation::Handler for State {
    fn feedback(&mut self, cx: Cx, surface: u32, callback: Obj) {
        match self.surfaces.get_mut(&(cx.client, surface)) {
            Some(s) => s.pending.feedbacks.push(callback.id),
            None => crate::surface::discard(self, cx.client, vec![callback.id]),
        }
    }
}

impl wp_presentation_feedback::Handler for State {}

impl wl_callback::Handler for State {}

impl zwp_idle_inhibit_manager_v1::Handler for State {
    fn create_inhibitor(&mut self, cx: Cx, id: Obj, surface: u32) {
        if let Some(s) = self.surfaces.get_mut(&(cx.client, surface)) {
            s.inhibitors += 1;
            self.client(cx.client).set_data(id.id, surface);
        }
    }
}

impl zwp_idle_inhibitor_v1::Handler for State {}

impl wl_region::Handler for State {
    fn add(&mut self, cx: Cx, x: i32, y: i32, width: i32, height: i32) {
        if let Some(Data::Region(r)) = self.data_mut(cx.key()) {
            r.add(Rect::new(x, y, width, height));
        }
    }

    fn subtract(&mut self, cx: Cx, x: i32, y: i32, width: i32, height: i32) {
        if let Some(Data::Region(r)) = self.data_mut(cx.key()) {
            r.subtract(Rect::new(x, y, width, height));
        }
    }
}

pub fn region(st: &mut State, c: ClientId, region: Option<u32>) -> Option<geom::Region> {
    region.map(|r| match st.data_mut((c, r)) {
        Some(Data::Region(region)) => region.clone(),
        _ => geom::Region::default(),
    })
}

impl wl_compositor::Handler for State {
    fn create_surface(&mut self, cx: Cx, id: Obj) {
        self.surfaces.insert((cx.client, id.id), Surface::new(cx.obj.version));
    }

    fn create_region(&mut self, cx: Cx, id: Obj) {
        self.set((cx.client, id.id), Data::Region(geom::Region::default()));
    }
}

impl wl_shm::Handler for State {
    fn create_pool(&mut self, cx: Cx, id: Obj, fd: OwnedFd, size: i32) -> Res {
        let count = self.client(cx.client).pools.clone();
        let pool = Pool::new(cx.obj.id, fd, size, count)?;
        self.set((cx.client, id.id), Data::Pool(Rc::new(pool)));
        Ok(())
    }
}

impl wl_shm_pool::Handler for State {
    fn create_buffer(
        &mut self,
        cx: Cx,
        id: Obj,
        offset: i32,
        width: i32,
        height: i32,
        stride: i32,
        format: u32,
    ) -> Res {
        let Some(Data::Pool(pool)) = self.data_mut(cx.key()) else { return Ok(()) };
        let pool = pool.clone();
        let shm =
            ShmBuffer::new(pool, offset, (width, height), stride, format).map_err(|(code, e)| cx.error(code, e))?;
        add_buffer(self, cx, id, buffer::Data::Shm(shm));
        Ok(())
    }

    fn resize(&mut self, cx: Cx, size: i32) -> Res {
        let Some(Data::Pool(pool)) = self.data_mut(cx.key()) else { return Ok(()) };
        pool.resize(size).map_err(|(code, e)| cx.error(code, e))
    }
}

impl wl_fixes::Handler for State {
    fn destroy_registry(&mut self, cx: Cx, registry: u32) -> Res {
        check_registry(self, cx, registry)?;
        crate::client::remove_object(self, cx.client, registry);
        Ok(())
    }

    fn ack_global_remove(&mut self, cx: Cx, registry: u32, name: u32) -> Res {
        check_registry(self, cx, registry)?;
        if name < output::GLOBAL_BASE {
            let message = format!("global {name} was not removed");
            return Err(cx.error(wl_fixes::error::INVALID_ACK_REMOVE, message));
        }
        Ok(())
    }
}

fn check_registry(st: &mut State, cx: Cx, registry: u32) -> Res {
    match st.client(cx.client).get_of(registry, Interface::WlRegistry) {
        Some(_) => Ok(()),
        None => Err(Error::new(DISPLAY.id, wl_display::error::INVALID_OBJECT, format!("{registry} is not a registry"))),
    }
}

impl wp_single_pixel_buffer_manager_v1::Handler for State {
    fn create_u32_rgba_buffer(&mut self, cx: Cx, id: Obj, r: u32, g: u32, b: u32, a: u32) {
        let pixel = [a, r, g, b].into_iter().fold(0, |argb, v| argb << 8 | v >> 24);
        let pixels = gfx::Pixels { data: vec![pixel], width: 1, height: 1 };
        add_buffer(self, cx, id, buffer::Data::Pixel(pixels));
    }
}

fn add_buffer(st: &mut State, cx: Cx, id: Obj, data: buffer::Data) {
    let buffer = Buffer::new(id.id, data, None, st.garbage.clone());
    st.set((cx.client, id.id), Data::Buffer(Rc::new(buffer)));
}

pub const KDE_SERVER_SIDE: u32 = 2;

impl wl_buffer::Handler for State {}
