mod config;
mod draw;
mod menu;
mod status;
mod tray;
mod workspaces;

use config::{Config, Position};
use menu::{Level, Menu, Open, Place};
use smithay_client_toolkit::{
    compositor::{CompositorHandler, CompositorState},
    delegate_dispatch2, delegate_registry,
    dispatch2::Dispatch2,
    output::{OutputHandler, OutputState},
    reexports::{
        calloop::{EventLoop, Interest, Mode, PostAction, channel, generic::Generic},
        calloop_wayland_source::WaylandSource,
        client::{
            Connection, QueueHandle,
            globals::registry_queue_init,
            protocol::{wl_output, wl_pointer, wl_seat, wl_surface},
        },
    },
    registry::{ProvidesRegistryState, RegistryState},
    registry_handlers,
    seat::{
        Capability, SeatHandler, SeatState,
        pointer::{AxisScroll, PointerEvent, PointerEventKind, PointerHandler},
    },
    shell::{
        WaylandSurface,
        wlr_layer::{
            Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface, LayerSurfaceConfigure,
        },
        xdg::{
            XdgShell,
            popup::{Popup, PopupConfigure, PopupHandler},
            window::{Window, WindowConfigure, WindowHandler},
        },
    },
    shm::{Shm, ShmHandler, slot::SlotPool},
};
use status::{Click, Status};
use std::{io::ErrorKind, os::fd::AsFd, sync::Arc};
use tray::{Entry, Item, Tray};
use wayland_protocols::{
    ext::workspace::v1::client::{ext_workspace_handle_v1::ExtWorkspaceHandleV1, ext_workspace_manager_v1},
    wp::{
        fractional_scale::v1::client::{
            wp_fractional_scale_manager_v1::WpFractionalScaleManagerV1,
            wp_fractional_scale_v1::{self, WpFractionalScaleV1},
        },
        viewporter::client::{wp_viewport::WpViewport, wp_viewporter::WpViewporter},
    },
};
use workspaces::Workspaces;

#[derive(Clone)]
pub enum Target {
    Workspace(ExtWorkspaceHandleV1),
    Block(usize),
    Tray(usize),
}

pub struct Bar {
    output: wl_output::WlOutput,
    layer: LayerSurface,
    fractional: Option<WpFractionalScaleV1>,
    viewport: Option<WpViewport>,
    width: u32,
    scale: u32,
    configured: bool,
    regions: Vec<(f64, f64, Target)>,
}

pub struct App {
    registry: RegistryState,
    outputs: OutputState,
    seats: SeatState,
    compositor: CompositorState,
    shm: Shm,
    layer_shell: LayerShell,
    fractional: Option<WpFractionalScaleManagerV1>,
    viewporter: Option<WpViewporter>,
    pool: SlotPool,
    bars: Vec<Bar>,
    pointer: Option<wl_pointer::WlPointer>,
    status: Option<Status>,
    workspaces: Workspaces,
    dirty: bool,
    qh: QueueHandle<App>,
    xdg: XdgShell,
    seat: Option<wl_seat::WlSeat>,
    serial: u32,
    tray: Option<Tray>,
    items: Vec<Item>,
    icons: Vec<(Arc<Vec<u8>>, cairo::ImageSurface)>,
    scroll: [f64; 2],
    menu: Option<Menu>,
    anchor: Option<(String, usize, (f64, f64))>,
    config: Config,
}

impl App {
    fn bar(&self, surface: &wl_surface::WlSurface) -> Option<usize> {
        self.bars.iter().position(|b| b.layer.wl_surface() == surface)
    }

    fn click(&mut self, event: &PointerEvent, button: u32) {
        let Some(index) = self.bar(&event.surface) else { return };
        let bar = &self.bars[index];
        let (x, y) = event.position;
        let Some((start, end, target)) = bar.regions.iter().find(|r| (r.0..r.1).contains(&x)).cloned() else {
            return;
        };
        match target {
            Target::Workspace(handle) if button == 1 => self.workspaces.activate(&handle),
            Target::Workspace(_) => {}
            Target::Block(i) => {
                let size = (end - start, f64::from(self.config.height));
                let click = Click { button, position: (x, y), relative: (x - start, y), size };
                self.status.iter_mut().for_each(|s| s.click(i, click));
            }
            Target::Tray(i) => {
                let (Some(tray), Some(item)) = (&self.tray, self.items.get(i).filter(|i| !i.passive)) else { return };
                self.anchor = Some((item.id.clone(), index, (start, end)));
                let at = (x as i32, y as i32);
                match button {
                    1 if item.is_menu && tray.menu(item) => {}
                    1 => tray.activate(item, "Activate", at),
                    2 => tray.activate(item, "SecondaryActivate", at),
                    3 if !tray.menu(item) => tray.activate(item, "ContextMenu", at),
                    4 | 5 => tray.scroll(item, if button == 4 { -1 } else { 1 }, "vertical"),
                    6 | 7 => tray.scroll(item, if button == 6 { -1 } else { 1 }, "horizontal"),
                    _ => {}
                }
            }
        }
    }

    fn tray_event(&mut self, event: tray::Event) {
        match event {
            tray::Event::Items(items) => {
                let old = std::mem::take(&mut self.icons);
                let cached =
                    |icon: &tray::Icon| old.iter().find(|(d, _)| Arc::ptr_eq(d, &icon.data)).map(|c| c.1.clone());
                let icon = |icon: &tray::Icon| Some((icon.data.clone(), cached(icon).or_else(|| draw::icon(icon))?));
                self.icons = items.iter().filter_map(|i| i.icon.as_ref()).filter_map(icon).collect();
                (self.items, self.dirty) = (items, true);
            }
            tray::Event::Menu(id, entries) => self.open_menu(id, entries),
            tray::Event::Submenu(id, parent, entries) => {
                let Some(menu) = &mut self.menu else { return };
                let pending = menu.pending.take();
                let Some((_, depth, row)) =
                    pending.filter(|p| menu.item == id && p.0 == parent && menu.levels.len() == p.1 + 1)
                else {
                    return;
                };
                if !entries.is_empty() {
                    self.open_level(depth, row, entries);
                }
            }
        }
    }

    fn opener(&self) -> Open<'_> {
        Open {
            qh: &self.qh,
            xdg: &self.xdg,
            viewporter: self.viewporter.as_ref(),
            surface: self.compositor.create_surface(&self.qh),
        }
    }

    fn open_menu(&mut self, id: String, entries: Vec<Entry>) {
        let Some((anchor, index, (start, end))) = self.anchor.take().filter(|a| a.0 == id && !entries.is_empty())
        else {
            return;
        };
        self.close_menu();
        let Some(bar) = self.bars.get(index) else { return };
        let rect = (start as i32, 0, (end - start) as i32, self.config.height as i32);
        let place = if self.config.position == Position::Top { Place::Down } else { Place::Up };
        let Some(level) = Level::open(self.opener(), None, entries, rect, place) else { return };
        bar.layer.get_popup(level.popup());
        self.seat.iter().for_each(|seat| level.grab(seat, self.serial));
        level.commit();
        self.menu = Some(Menu { item: anchor, scale: bar.scale, levels: vec![level], pending: None });
    }

    fn open_level(&mut self, depth: usize, row: usize, entries: Vec<Entry>) {
        let Some(parent) = self.menu.as_ref().and_then(|m| m.levels.get(depth)) else { return };
        let (rect, xdg) = (parent.row(row), parent.xdg_surface().clone());
        let Some(level) = Level::open(self.opener(), Some(&xdg), entries, rect, Place::Side) else { return };
        self.seat.iter().for_each(|seat| level.grab(seat, self.serial));
        level.commit();
        if let Some(menu) = &mut self.menu {
            menu.levels.push(level);
        }
    }

    fn close_menu(&mut self) {
        for level in self.menu.take().into_iter().flat_map(|m| m.levels).rev() {
            level.close();
        }
    }

    fn menu_pointer(&mut self, event: &PointerEvent) -> bool {
        let Some(menu) = &mut self.menu else { return false };
        let Some(depth) = menu.levels.iter().position(|l| l.surface() == &event.surface) else { return false };
        let row = menu.levels[depth].at(event.position.1);
        match event.kind {
            PointerEventKind::Enter { .. } | PointerEventKind::Motion { .. } => {
                if menu.levels[depth].hover(row) {
                    menu.levels[depth].draw(&mut self.pool, menu.scale, &self.config.colors);
                }
            }
            PointerEventKind::Leave { .. } => {
                if menu.levels[depth].hover(None) {
                    menu.levels[depth].draw(&mut self.pool, menu.scale, &self.config.colors);
                }
            }
            PointerEventKind::Press { serial, .. } => {
                self.serial = serial;
                let Some(row) = row else { return true };
                let Some(entry) = menu.levels[depth].entry(row).filter(|e| e.enabled && !e.separator).cloned() else {
                    return true;
                };
                for level in menu.levels.drain(depth + 1..).rev() {
                    level.close();
                }
                menu.pending = None;
                let item = self.items.iter().find(|i| i.id == menu.item).cloned();
                if !entry.children.is_empty() {
                    self.open_level(depth, row, entry.children);
                } else if entry.submenu {
                    menu.pending = Some((entry.id, depth, row));
                    if let (Some(tray), Some(item)) = (&self.tray, item) {
                        tray.submenu(&item, entry.id);
                    }
                } else {
                    if let (Some(tray), Some(item)) = (&self.tray, item) {
                        tray.click(&item, entry.id);
                    }
                    self.close_menu();
                }
            }
            _ => {}
        }
        true
    }
}

fn main() {
    env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("info,zbus=warn,tracing=warn")).init();
    let config = args().and_then(Config::load).unwrap_or_else(|e| {
        log::error!("{e}");
        std::process::exit(2);
    });
    draw::set_font(&config.font);
    if let Err(e) = run(config) {
        log::error!("{e}");
        std::process::exit(1);
    }
}

fn args() -> Result<Option<std::path::PathBuf>, String> {
    const USAGE: &str = "usage: i3bar-wl [--config PATH]";
    let (mut args, mut path) = (std::env::args_os().skip(1), None);
    while let Some(arg) = args.next() {
        match arg.to_str() {
            Some("-c" | "--config") => {
                path = Some(args.next().ok_or(format!("--config needs a path; {USAGE}"))?.into())
            }
            Some("-h" | "--help") => {
                println!("{USAGE}");
                std::process::exit(0);
            }
            _ => return Err(format!("unexpected argument {arg:?}; {USAGE}")),
        }
    }
    Ok(path)
}

fn run(config: Config) -> Result<(), Box<dyn std::error::Error>> {
    let conn = Connection::connect_to_env()?;
    let (globals, queue) = registry_queue_init(&conn)?;
    let qh = queue.handle();
    let mut event_loop: EventLoop<App> = EventLoop::try_new()?;
    WaylandSource::new(conn, queue).insert(event_loop.handle()).map_err(|e| e.error)?;
    let shm = Shm::bind(&globals, &qh).map_err(|e| format!("wl_shm: {e}"))?;
    let pool = SlotPool::new(256 * 1024, &shm)?;
    let mut app = App {
        registry: RegistryState::new(&globals),
        outputs: OutputState::new(&globals, &qh),
        seats: SeatState::new(&globals, &qh),
        compositor: CompositorState::bind(&globals, &qh).map_err(|e| format!("wl_compositor: {e}"))?,
        layer_shell: LayerShell::bind(&globals, &qh)
            .map_err(|e| format!("zwlr_layer_shell_v1: {e} (is i3bar-wl in client_caps?)"))?,
        fractional: globals.bind(&qh, 1..=1, Silent).ok(),
        viewporter: globals.bind(&qh, 1..=1, Silent).ok(),
        shm,
        pool,
        bars: vec![],
        pointer: None,
        status: None,
        workspaces: Workspaces::default(),
        dirty: false,
        qh: qh.clone(),
        xdg: XdgShell::bind(&globals, &qh).map_err(|e| format!("xdg_wm_base: {e}"))?,
        seat: None,
        serial: 0,
        tray: None,
        items: vec![],
        icons: vec![],
        scroll: [0.0; 2],
        menu: None,
        anchor: None,
        config,
    };
    let (sender, events) = channel::channel();
    event_loop.handle().insert_source(events, |event, _, app: &mut App| {
        if let channel::Event::Msg(event) = event {
            app.tray_event(event);
        }
    })?;
    match Tray::start(sender) {
        Ok(tray) => app.tray = Some(tray),
        Err(e) => log::warn!("no tray: {e}"),
    }
    app.workspaces.manager =
        globals.bind::<ext_workspace_manager_v1::ExtWorkspaceManagerV1, _, _>(&qh, 1..=1, workspaces::Data).ok();
    match Status::spawn(&app.config.command, &app.config.env) {
        Ok((status, stdout)) => {
            rustix::io::ioctl_fionbio(&stdout, true)?;
            app.status = Some(status);
            let source = Generic::new(stdout, Interest::READ, Mode::Level);
            event_loop.handle().insert_source(source, |_, stdout, app| {
                let mut buf = [0; 4096];
                loop {
                    match rustix::io::read(stdout.as_fd(), &mut buf) {
                        Ok(n) if n > 0 => app.dirty |= app.status.as_mut().is_some_and(|s| s.feed(&buf[..n])),
                        Err(e) if e.kind() == ErrorKind::WouldBlock => return Ok(PostAction::Continue),
                        Err(e) if e.kind() == ErrorKind::Interrupted => {}
                        result => {
                            match result {
                                Err(e) => log::warn!("status command: {e}"),
                                Ok(_) => log::warn!("status command exited"),
                            }
                            app.status.take().into_iter().for_each(Status::stop);
                            app.dirty = true;
                            return Ok(PostAction::Remove);
                        }
                    }
                }
            })?;
        }
        Err(e) => log::warn!("cannot start {}: {e}", app.config.command.join(" ")),
    }
    loop {
        event_loop.dispatch(None, &mut app)?;
        if std::mem::take(&mut app.dirty) {
            for index in 0..app.bars.len() {
                draw::draw(&mut app, index);
            }
        }
    }
}

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.outputs
    }

    fn new_output(&mut self, _: &Connection, qh: &QueueHandle<Self>, output: wl_output::WlOutput) {
        let surface = self.compositor.create_surface(qh);
        let fractional = self
            .viewporter
            .as_ref()
            .and(self.fractional.as_ref())
            .map(|m| m.get_fractional_scale(&surface, qh, Scale(surface.clone())));
        let viewport = self.viewporter.as_ref().map(|v| v.get_viewport(&surface, qh, Silent));
        let layer = self.layer_shell.create_layer_surface(qh, surface, Layer::Top, Some("bar"), Some(&output));
        let edge = if self.config.position == Position::Top { Anchor::TOP } else { Anchor::BOTTOM };
        layer.set_anchor(edge | Anchor::LEFT | Anchor::RIGHT);
        layer.set_size(0, self.config.height);
        layer.set_exclusive_zone(self.config.height as i32);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);
        layer.commit();
        let (width, scale, configured, regions) = (0, 120, false, vec![]);
        self.bars.push(Bar { output, layer, fractional, viewport, width, scale, configured, regions });
    }

    fn update_output(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_output::WlOutput) {}

    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<Self>, output: wl_output::WlOutput) {
        for bar in self.bars.extract_if(.., |b| b.output == output) {
            bar.fractional.iter().for_each(|f| f.destroy());
            bar.viewport.iter().for_each(|v| v.destroy());
        }
    }
}

impl LayerShellHandler for App {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<Self>, layer: &LayerSurface) {
        self.bars.retain(|b| &b.layer != layer);
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        layer: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        if let Some(bar) = self.bars.iter_mut().find(|b| &b.layer == layer) {
            (bar.width, bar.configured) = (configure.new_size.0, true);
            self.dirty = true;
        }
    }
}

impl CompositorHandler for App {
    fn scale_factor_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        surface: &wl_surface::WlSurface,
        factor: i32,
    ) {
        if let Some(bar) = self.bars.iter_mut().find(|b| b.layer.wl_surface() == surface && b.fractional.is_none()) {
            (bar.scale, self.dirty) = (factor.max(1) as u32 * 120, true);
        }
    }

    fn transform_changed(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: wl_output::Transform,
    ) {
    }

    fn frame(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &wl_surface::WlSurface, _: u32) {}

    fn surface_enter(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }

    fn surface_leave(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_surface::WlSurface,
        _: &wl_output::WlOutput,
    ) {
    }
}

impl SeatHandler for App {
    fn seat_state(&mut self) -> &mut SeatState {
        &mut self.seats
    }

    fn new_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, seat: wl_seat::WlSeat) {
        self.seat.get_or_insert(seat);
    }

    fn new_capability(
        &mut self,
        _: &Connection,
        qh: &QueueHandle<Self>,
        seat: wl_seat::WlSeat,
        capability: Capability,
    ) {
        if capability == Capability::Pointer && self.pointer.is_none() {
            self.pointer = self.seats.get_pointer(qh, &seat).ok();
        }
    }

    fn remove_capability(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat, capability: Capability) {
        if capability == Capability::Pointer {
            self.pointer.take().iter().for_each(|p| p.release());
        }
    }

    fn remove_seat(&mut self, _: &Connection, _: &QueueHandle<Self>, _: wl_seat::WlSeat) {}
}

impl PointerHandler for App {
    fn pointer_frame(
        &mut self,
        _: &Connection,
        _: &QueueHandle<Self>,
        _: &wl_pointer::WlPointer,
        events: &[PointerEvent],
    ) {
        for event in events {
            if self.menu_pointer(event) {
                continue;
            }
            match &event.kind {
                PointerEventKind::Press { button, serial, .. } => {
                    self.serial = *serial;
                    let button = match *button {
                        0x110 => 1,
                        0x112 => 2,
                        0x111 => 3,
                        0x113 => 8,
                        0x114 => 9,
                        _ => continue,
                    };
                    self.click(event, button);
                }
                PointerEventKind::Axis { vertical, horizontal, .. } => {
                    let delta = |a: &AxisScroll| match (a.value120, a.discrete) {
                        (0, 0) => a.absolute * 12.0,
                        (0, d) => f64::from(d * 120),
                        (v, _) => f64::from(v),
                    };
                    let mut scroll = [self.scroll[0] + delta(vertical), self.scroll[1] + delta(horizontal)];
                    for (acc, (back, forward)) in scroll.iter_mut().zip([(4, 5), (6, 7)]) {
                        while acc.abs() >= 120.0 {
                            self.click(event, if *acc < 0.0 { back } else { forward });
                            *acc -= 120f64.copysign(*acc);
                        }
                    }
                    self.scroll = scroll;
                }
                _ => {}
            }
        }
    }
}

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl PopupHandler for App {
    fn configure(&mut self, _: &Connection, _: &QueueHandle<Self>, popup: &Popup, config: PopupConfigure) {
        let Some(menu) = &mut self.menu else { return };
        if let Some(level) = menu.levels.iter_mut().find(|l| l.is(popup)) {
            level.configure(&config);
            level.draw(&mut self.pool, menu.scale, &self.config.colors);
        }
    }

    fn done(&mut self, _: &Connection, _: &QueueHandle<Self>, popup: &Popup) {
        if self.menu.as_ref().is_some_and(|m| m.levels.iter().any(|l| l.is(popup))) {
            self.close_menu();
        }
    }
}

impl WindowHandler for App {
    fn request_close(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window) {}

    fn configure(&mut self, _: &Connection, _: &QueueHandle<Self>, _: &Window, _: WindowConfigure, _: u32) {}
}

pub struct Scale(wl_surface::WlSurface);

impl Dispatch2<WpFractionalScaleV1, App> for Scale {
    fn event(
        &self,
        app: &mut App,
        _: &WpFractionalScaleV1,
        event: wp_fractional_scale_v1::Event,
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
        let wp_fractional_scale_v1::Event::PreferredScale { scale } = event else { return };
        if let Some(index) = app.bar(&self.0) {
            app.bars[index].scale = scale;
            app.dirty = true;
        }
    }
}

pub struct Silent;

impl<I: smithay_client_toolkit::reexports::client::Proxy> Dispatch2<I, App> for Silent {
    fn event(&self, _: &mut App, _: &I, _: I::Event, _: &Connection, _: &QueueHandle<App>) {}
}

delegate_registry!(App);
delegate_dispatch2!(App);

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry
    }

    registry_handlers![OutputState, SeatState];
}
