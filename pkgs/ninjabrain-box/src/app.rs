//! The Wayland side: an overlay-layer surface that appears and disappears on
//! command, and everything hanging off the same event loop.

use std::rc::Rc;

use smithay_client_toolkit::compositor::{CompositorHandler, CompositorState};
use smithay_client_toolkit::output::{OutputHandler, OutputState};
use smithay_client_toolkit::registry::{ProvidesRegistryState, RegistryState};
use smithay_client_toolkit::shell::wlr_layer::{
    Anchor, KeyboardInteractivity, Layer, LayerShell, LayerShellHandler, LayerSurface,
    LayerSurfaceConfigure,
};
use smithay_client_toolkit::shell::WaylandSurface;
use smithay_client_toolkit::shm::slot::SlotPool;
use smithay_client_toolkit::shm::{Shm, ShmHandler};
use smithay_client_toolkit::{
    delegate_compositor, delegate_layer, delegate_output, delegate_registry, delegate_shm,
    registry_handlers,
};
use wayland_client::globals::GlobalList;
use wayland_client::protocol::{wl_output::WlOutput, wl_seat::WlSeat, wl_shm, wl_surface};
use wayland_client::{Connection, Dispatch, Proxy, QueueHandle};

use crate::action::Action;
use crate::clipboard::Clipboard;
use crate::config::{Config, Edge};
use crate::model::Stronghold;
use crate::render;
use crate::text::Text;
use crate::toplevel::Focus;
use crate::xserver::XServer;

pub struct App {
    registry_state: RegistryState,
    output_state: OutputState,
    shm: Shm,
    compositor: CompositorState,
    layer_shell: Option<LayerShell>,
    pool: SlotPool,
    queue: QueueHandle<App>,

    pub config: Config,
    text: Text,
    state: Stronghold,

    layer: Option<LayerSurface>,
    /// The size the compositor last agreed to.
    size: (u32, u32),

    /// What the user last asked for, before the focus filter has its say.
    wanted: bool,
    show_throws: bool,

    pub clipboard: Option<Clipboard>,
    pub clipboard_sink: calloop::channel::Sender<String>,
    pub focus: Option<Focus>,

    x: Rc<XServer>,
    pub exit: bool,
}

impl App {
    pub fn new(
        globals: &GlobalList,
        queue: &QueueHandle<App>,
        config: Config,
        text: Text,
        x: Rc<XServer>,
        clipboard_sink: calloop::channel::Sender<String>,
    ) -> Result<App, String> {
        let shm = Shm::bind(globals, queue).map_err(|error| format!("wl_shm: {error}"))?;
        let pool = SlotPool::new(256 * 256 * 4, &shm).map_err(|error| format!("shm pool: {error}"))?;
        let compositor = CompositorState::bind(globals, queue)
            .map_err(|error| format!("wl_compositor: {error}"))?;
        let layer_shell = LayerShell::bind(globals, queue).ok();
        if layer_shell.is_none() {
            eprintln!(
                "ninjabrain-box: the compositor has no zwlr_layer_shell_v1, \
                 so there is nowhere to put the window"
            );
        }

        let clipboard = globals
            .bind::<WlSeat, _, _>(queue, 1..=9, ())
            .ok()
            .and_then(|seat| match Clipboard::new(globals, queue, &seat) {
                Ok(clipboard) => Some(clipboard),
                Err(reason) => {
                    eprintln!("ninjabrain-box: {reason}");
                    None
                }
            });

        let focus = Focus::new(globals, queue);
        if focus.is_none() && !config.behavior.only_when_focused.is_empty() {
            eprintln!(
                "ninjabrain-box: the compositor has no \
                 zwlr_foreign_toplevel_manager_v1, so only-when-focused is ignored"
            );
        }

        let wanted = !config.behavior.start_hidden;
        let show_throws = config.behavior.start_with_throws;
        Ok(App {
            registry_state: RegistryState::new(globals),
            output_state: OutputState::new(globals, queue),
            shm,
            compositor,
            layer_shell,
            pool,
            queue: queue.clone(),
            config,
            text,
            state: Stronghold::default(),
            layer: None,
            size: (1, 1),
            wanted,
            show_throws,
            clipboard,
            clipboard_sink,
            focus,
            x,
            exit: false,
        })
    }

    /// New state from the bot.
    pub fn update(&mut self, state: Stronghold) {
        self.state = state;
        self.redraw();
    }

    pub fn act(&mut self, action: Action) {
        match action {
            Action::Bot(action) => self.x.press(action.keysym),
            Action::Show => {
                self.wanted = true;
                self.refresh_visibility();
            }
            Action::Hide => {
                self.wanted = false;
                self.refresh_visibility();
            }
            Action::Toggle => {
                let action = if self.wanted { Action::Hide } else { Action::Show };
                self.act(action);
            }
            Action::ShowThrows => {
                self.show_throws = true;
                self.redraw();
            }
            Action::HideThrows => {
                self.show_throws = false;
                self.redraw();
            }
            Action::ToggleThrows => {
                let action = if self.show_throws {
                    Action::HideThrows
                } else {
                    Action::ShowThrows
                };
                self.act(action);
            }
            Action::Quit => self.exit = true,
            Action::Reload => self.reload(),
        }
    }

    /// Re-reads the configuration and applies everything that can be applied
    /// without starting over.
    fn reload(&mut self) {
        let config = match Config::load() {
            Ok(config) => config,
            Err(reason) => return eprintln!("ninjabrain-box: {reason}"),
        };

        match crate::font_bytes(&config)
            .and_then(|bytes| Text::new(&bytes, config.window.font_size))
        {
            Ok(text) => self.text = text,
            // Keep drawing in the old one rather than stop drawing.
            Err(reason) => eprintln!("ninjabrain-box: keeping the old font: {reason}"),
        }

        // The bot reads its preferences once, at startup. Writing them now
        // means a restart picks the change up; saying so means nobody waits
        // for something that is not going to happen.
        if config.bot.settings != self.config.bot.settings {
            match crate::prefs::write(&crate::state_directory(), &config.bot.settings) {
                Ok(()) => eprintln!(
                    "ninjabrain-box: the bot's settings changed; restart to apply them"
                ),
                Err(reason) => eprintln!("ninjabrain-box: {reason}"),
            }
        }

        self.config = config;
        self.rebuild();
    }

    /// Whether the focus filter, if any, is satisfied right now.
    fn focus_allows(&self) -> bool {
        let patterns = &self.config.behavior.only_when_focused;
        match (patterns.is_empty(), self.focus.as_ref()) {
            (true, _) | (false, None) => true,
            (false, Some(focus)) => focus.matches(patterns),
        }
    }

    /// Brings the surface into line with what should be on screen.
    pub fn refresh_visibility(&mut self) {
        let should_show = self.wanted && self.focus_allows() && self.layer_shell.is_some();
        match (should_show, self.layer.is_some()) {
            (true, false) => self.create_layer(),
            (false, true) => self.layer = None,
            _ => {}
        }
    }

    /// Tears the surface down and puts it back, for a configuration change.
    fn rebuild(&mut self) {
        self.layer = None;
        self.refresh_visibility();
    }

    fn create_layer(&mut self) {
        let Some(shell) = self.layer_shell.as_ref() else {
            return;
        };
        let surface = self.compositor.create_surface(&self.queue);
        let output = self.chosen_output();
        let layer = shell.create_layer_surface(
            &self.queue,
            surface,
            Layer::Overlay,
            Some("ninjabrain-box"),
            output.as_ref(),
        );

        let mut anchor = Anchor::empty();
        for edge in &self.config.window.anchor {
            anchor |= match edge {
                Edge::Top => Anchor::TOP,
                Edge::Bottom => Anchor::BOTTOM,
                Edge::Left => Anchor::LEFT,
                Edge::Right => Anchor::RIGHT,
            };
        }
        layer.set_anchor(anchor);
        let [top, right, bottom, left] = self.config.window.margin;
        layer.set_margin(top, right, bottom, left);
        // Neither reserve space nor be pushed around by anything that does.
        layer.set_exclusive_zone(-1);
        layer.set_keyboard_interactivity(KeyboardInteractivity::None);

        let layout = self.layout();
        self.size = (layout.width, layout.height);
        layer.set_size(layout.width, layout.height);
        layer.commit();
        self.layer = Some(layer);
    }

    /// The output the configuration names, if the compositor has one by that
    /// name. Saying so matters: a name that matches nothing looks exactly like
    /// no name at all, and the window quietly turns up wherever.
    fn chosen_output(&self) -> Option<WlOutput> {
        let wanted = self.config.window.output.as_ref()?;
        let found = self.output_state.outputs().find(|output| {
            self.output_state
                .info(output)
                .and_then(|info| info.name)
                .is_some_and(|name| &name == wanted)
        });
        if found.is_none() {
            let names: Vec<String> = self
                .output_state
                .outputs()
                .filter_map(|output| self.output_state.info(&output)?.name)
                .collect();
            eprintln!(
                "ninjabrain-box: no output called {wanted}; the compositor offers {}. \
                 Letting it choose.",
                if names.is_empty() { "none".to_owned() } else { names.join(", ") }
            );
        }
        found
    }

    fn layout(&mut self) -> render::Layout {
        render::layout(&self.state, &self.config, &mut self.text, self.show_throws)
    }

    /// Re-measures, and asks for a new size if the content no longer fits.
    fn redraw(&mut self) {
        if self.layer.is_none() {
            return;
        }
        let layout = self.layout();
        if (layout.width, layout.height) != self.size {
            self.size = (layout.width, layout.height);
            if let Some(layer) = self.layer.as_ref() {
                layer.set_size(layout.width, layout.height);
                layer.commit();
            }
            // The configure that follows will draw it.
            return;
        }
        self.paint(&layout);
    }

    fn paint(&mut self, layout: &render::Layout) {
        let Some(layer) = self.layer.as_ref() else {
            return;
        };
        let (width, height) = (self.size.0.max(1) as i32, self.size.1.max(1) as i32);
        let stride = width * 4;
        let Ok((buffer, pixels)) =
            self.pool
                .create_buffer(width, height, stride, wl_shm::Format::Argb8888)
        else {
            return;
        };
        pixels.fill(0);
        let mut canvas = render::Canvas {
            pixels,
            width,
            height,
        };
        render::draw(layout, &mut canvas, &mut self.text, &self.config);

        let surface = layer.wl_surface();
        surface.damage_buffer(0, 0, width, height);
        if buffer.attach_to(surface).is_ok() {
            surface.commit();
        }
    }

}

impl CompositorHandler for App {
    fn scale_factor_changed(&mut self, _: &Connection, _: &QueueHandle<App>, _: &wl_surface::WlSurface, _: i32) {}
    fn transform_changed(&mut self, _: &Connection, _: &QueueHandle<App>, _: &wl_surface::WlSurface, _: wl_output::Transform) {}
    fn frame(&mut self, _: &Connection, _: &QueueHandle<App>, _: &wl_surface::WlSurface, _: u32) {}
    fn surface_enter(&mut self, _: &Connection, _: &QueueHandle<App>, _: &wl_surface::WlSurface, _: &WlOutput) {}
    fn surface_leave(&mut self, _: &Connection, _: &QueueHandle<App>, _: &wl_surface::WlSurface, _: &WlOutput) {}
}

use wayland_client::protocol::wl_output;

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<App>, _: WlOutput) {
        // The wanted output may only just have appeared.
        if self.layer.is_some() && self.config.window.output.is_some() {
            self.rebuild();
        }
    }
    fn update_output(&mut self, _: &Connection, _: &QueueHandle<App>, _: WlOutput) {}
    fn output_destroyed(&mut self, _: &Connection, _: &QueueHandle<App>, _: WlOutput) {}
}

impl ShmHandler for App {
    fn shm_state(&mut self) -> &mut Shm {
        &mut self.shm
    }
}

impl LayerShellHandler for App {
    fn closed(&mut self, _: &Connection, _: &QueueHandle<App>, _: &LayerSurface) {
        self.layer = None;
    }

    fn configure(
        &mut self,
        _: &Connection,
        _: &QueueHandle<App>,
        _: &LayerSurface,
        configure: LayerSurfaceConfigure,
        _: u32,
    ) {
        // A zero in either axis means "you decide", which we already have.
        let (width, height) = configure.new_size;
        if width != 0 && height != 0 {
            self.size = (width, height);
        }
        let layout = self.layout();
        self.paint(&layout);
    }
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

impl Dispatch<WlSeat, ()> for App {
    fn event(
        _: &mut App,
        _: &WlSeat,
        _: <WlSeat as Proxy>::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
    }
}

delegate_compositor!(App);
delegate_output!(App);
delegate_shm!(App);
delegate_layer!(App);
delegate_registry!(App);
