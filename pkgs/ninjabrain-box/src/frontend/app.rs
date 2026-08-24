//! The Wayland side: an overlay-layer surface that appears and disappears on
//! command, and everything hanging off the same event loop.
//!
//! This reads a [`Control`] and draws it. Every request that changes anything
//! but what is on screen goes straight through to the box.

use anyhow::{Context, Result};
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
use wayland_client::protocol::wl_pointer::{self, WlPointer};
use wayland_client::protocol::wl_seat::{self, WlSeat};
use wayland_client::protocol::{wl_output, wl_output::WlOutput, wl_region, wl_shm, wl_surface};
use wayland_client::{Connection, Dispatch, QueueHandle};

use crate::action::{Request, Target};
use crate::config::{Config, Edge};
use crate::control::{Answer, Control};
use crate::model::Solution;

use super::clipboard::Clipboard;
use super::render;
use super::text::Text;
use super::toplevel::Focus;

/// What was last asked for, which overrides the focus rule.
///
/// `show`, `hide` and `toggle` override the rule; the override is spent when
/// the rule *becomes* satisfied again -- when a window it names takes focus.
/// With no focus rule there is nothing to become satisfied, so an override
/// simply stands until the next one, which is what anyone would expect of
/// `show` and `hide` on a window that follows nothing.
#[derive(Clone, Copy, Debug, PartialEq)]
enum Wanted {
    /// Follow the focus rule.
    Follow,
    Shown,
    Hidden,
}

impl Wanted {
    /// Spends the override if the focus rule has just become satisfied.
    fn settle(self, allowed: bool, was_allowed: bool) -> Wanted {
        match allowed && !was_allowed {
            true => Wanted::Follow,
            false => self,
        }
    }

    fn shows(self, allowed: bool) -> bool {
        match self {
            Wanted::Shown => true,
            Wanted::Hidden => false,
            Wanted::Follow => allowed,
        }
    }
}

pub struct App {
    registry_state: RegistryState,
    output_state: OutputState,
    shm: Shm,
    compositor: CompositorState,
    layer_shell: Option<LayerShell>,
    pool: SlotPool,
    queue: QueueHandle<App>,
    pointer: Option<WlPointer>,

    pub control: Control,
    config: Config,
    text: Text,

    layer: Option<LayerSurface>,
    /// The size the compositor last agreed to.
    size: (u32, u32),
    /// The layout the pixels on screen were drawn from, which is what hit
    /// testing runs against.
    layout: render::Layout,

    wanted: Wanted,
    /// Whether the focus rule was satisfied last time it was looked at, so
    /// that becoming satisfied can be told from staying so.
    was_allowed: bool,
    show_throws: bool,
    pointer_at: Option<(f64, f64)>,
    hovered: Option<usize>,

    pub clipboard: Option<Clipboard>,
    pub clipboard_sink: calloop::channel::Sender<String>,
    pub focus: Option<Focus>,
}

impl App {
    pub fn new(
        globals: &GlobalList,
        queue: &QueueHandle<App>,
        config: Config,
        text: Text,
        control: Control,
        clipboard_sink: calloop::channel::Sender<String>,
    ) -> Result<App> {
        let shm = Shm::bind(globals, queue).context("wl_shm")?;
        let pool =
            SlotPool::new(256 * 256 * 4, &shm).context("shm pool")?;
        let compositor = CompositorState::bind(globals, queue)
            .context("wl_compositor")?;
        let layer_shell = LayerShell::bind(globals, queue).ok();
        if layer_shell.is_none() {
            eprintln!(
                "ninjabrain-box: the compositor has no zwlr_layer_shell_v1, \
                 so there is nowhere to put the window"
            );
        }

        let seat = globals.bind::<WlSeat, _, _>(queue, 1..=9, ()).ok();
        let clipboard = seat
            .as_ref()
            .and_then(|seat| match Clipboard::new(globals, queue, seat) {
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

        let wanted = if config.behavior.start_hidden {
            Wanted::Hidden
        } else {
            Wanted::Follow
        };
        let show_throws = config.behavior.start_with_throws;
        Ok(App {
            registry_state: RegistryState::new(globals),
            output_state: OutputState::new(globals, queue),
            shm,
            compositor,
            layer_shell,
            pool,
            queue: queue.clone(),
            pointer: None,
            control,
            config,
            text,
            layer: None,
            size: (1, 1),
            layout: render::Layout::default(),
            wanted,
            was_allowed: false,
            show_throws,
            pointer_at: None,
            hovered: None,
            clipboard,
            clipboard_sink,
            focus,
        })
    }

    /// A fresh answer from the calculator.
    pub fn update(&mut self, solution: Solution) {
        self.control.absorb(solution);
        self.refresh_visibility();
        self.redraw();
    }

    /// Something arrived on the compositor's clipboard.
    pub fn on_clipboard(&mut self, text: &str) {
        // Most of what lands on a clipboard is not a measurement, so only a
        // line that was one and still failed is worth reporting.
        if crate::model::Measurement::parse(text).is_some() {
            if let Err(reason) = self.control.measure(text) {
                eprintln!("ninjabrain-box: {reason:#}");
            }
        }
        self.redraw();
    }

    /// Carries a plan along, and redraws if that changed anything visible.
    pub fn tick(&mut self) {
        let before = self.control.busy().is_some();
        self.control.tick();
        if before != self.control.busy().is_some() {
            self.redraw();
        }
    }

    pub fn act(&mut self, request: &Request) -> Result<Answer> {
        let answer = match request {
            Request::Show(target) => self.visibility(*target, Some(true)),
            Request::Hide(target) => self.visibility(*target, Some(false)),
            Request::Toggle(target) => self.visibility(*target, None),
            other => self.control.act(other)?,
        };
        self.redraw();
        Ok(answer)
    }

    /// `None` toggles.
    fn visibility(&mut self, target: Target, shown: Option<bool>) -> Answer {
        match target {
            Target::All => {
                let shown = shown.unwrap_or(!self.should_show());
                self.wanted = if shown { Wanted::Shown } else { Wanted::Hidden };
                self.refresh_visibility();
            }
            Target::Throws => self.show_throws = shown.unwrap_or(!self.show_throws),
        }
        Answer::Done
    }

    /// Whether the window is wanted on screen, before `auto-hide` has its say.
    fn should_show(&self) -> bool {
        self.wanted.shows(self.focus_allows())
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
        // Asking for the window overrides the focus rule, and that lasts until
        // a window the rule names is focused again -- so the override is spent
        // by the rule *becoming* satisfied, not by it being so.
        let allowed = self.focus_allows();
        self.wanted = self.wanted.settle(allowed, self.was_allowed);
        self.was_allowed = allowed;

        let idle = self.config.behavior.auto_hide
            && self.control.solution().is_idle()
            && self.control.busy().is_none();
        let should_show = self.wanted.shows(allowed) && !idle && self.layer_shell.is_some();
        match (should_show, self.layer.is_some()) {
            (true, false) => self.create_layer(),
            (false, true) => {
                self.layer = None;
                self.pointer_at = None;
                self.hovered = None;
            }
            _ => {}
        }
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
        self.layout = layout;
        layer.set_size(self.size.0, self.size.1);
        layer.commit();
        self.layer = Some(layer);
    }

    /// Accepts pointer input over the buttons and nowhere else.
    ///
    /// A surface takes input everywhere by default, and an overlay that does
    /// that eats clicks meant for the game underneath. Naming just the buttons
    /// keeps them pressable while the rest of the panel stays click-through.
    fn set_input_region(&self, layer: &LayerSurface) {
        let region = self.compositor.wl_compositor().create_region(&self.queue, ());
        for button in &self.layout.buttons {
            region.add(
                button.x.floor() as i32,
                button.y.floor() as i32,
                button.width.ceil() as i32,
                button.height.ceil() as i32,
            );
        }
        layer.wl_surface().set_input_region(Some(&region));
        region.destroy();
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
                if names.is_empty() {
                    "none".to_owned()
                } else {
                    names.join(", ")
                }
            );
        }
        found
    }

    fn layout(&mut self) -> render::Layout {
        let note = self.control.busy().map(|what| format!("{what}..."));
        let control = &self.control;
        let permits = |request: &Request| control.permits(request);
        let options = render::Options {
            throws: self.show_throws,
            note: note.as_deref(),
            permits: &permits,
        };
        render::layout(
            self.control.solution(),
            &self.config,
            &mut self.text,
            &options,
        )
    }

    /// Re-measures, and asks for a new size if the content no longer fits.
    fn redraw(&mut self) {
        if self.layer.is_none() {
            return;
        }
        let layout = self.layout();
        if (layout.width, layout.height) != self.size {
            self.size = (layout.width, layout.height);
            self.layout = layout;
            if let Some(layer) = self.layer.as_ref() {
                layer.set_size(self.size.0, self.size.1);
                layer.commit();
            }
            // The configure that follows will draw it.
            return;
        }
        self.layout = layout;
        self.repaint();
    }

    fn repaint(&mut self) {
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
        render::draw(
            &self.layout,
            pixels,
            (width as u32, height as u32),
            &mut self.text,
            &self.config,
            self.hovered,
        );

        self.set_input_region(layer);
        let surface = layer.wl_surface();
        surface.damage_buffer(0, 0, width, height);
        if buffer.attach_to(surface).is_ok() {
            surface.commit();
        }
    }

    /// The pointer moved, or left. Redraws only when the highlight changes, so
    /// that moving across the panel is not a stream of buffers.
    fn pointer_moved(&mut self, position: Option<(f64, f64)>) {
        self.pointer_at = position;
        let hovered = position.and_then(|(x, y)| {
            self.layout
                .buttons
                .iter()
                .position(|button| button.enabled && button.covers(x, y))
        });
        if hovered != self.hovered {
            self.hovered = hovered;
            self.repaint();
        }
    }

    fn pointer_pressed(&mut self) {
        let Some(request) = self
            .hovered
            .and_then(|index| self.layout.buttons.get(index))
            .filter(|button| button.enabled)
            .map(|button| button.request.clone())
        else {
            return;
        };
        if let Err(reason) = self.act(&request) {
            eprintln!("ninjabrain-box: {reason}");
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

impl OutputHandler for App {
    fn output_state(&mut self) -> &mut OutputState {
        &mut self.output_state
    }
    fn new_output(&mut self, _: &Connection, _: &QueueHandle<App>, _: WlOutput) {
        // The wanted output may only just have appeared.
        if self.layer.is_some() && self.config.window.output.is_some() {
            self.layer = None;
            self.refresh_visibility();
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
        self.layout = self.layout();
        self.repaint();
    }
}

impl ProvidesRegistryState for App {
    fn registry(&mut self) -> &mut RegistryState {
        &mut self.registry_state
    }
    registry_handlers![OutputState];
}

/// The seat is bound for its clipboard; the pointer is only taken when there
/// is something on the panel to press.
impl Dispatch<WlSeat, ()> for App {
    fn event(
        app: &mut App,
        seat: &WlSeat,
        event: wl_seat::Event,
        _: &(),
        _: &Connection,
        queue: &QueueHandle<App>,
    ) {
        let wl_seat::Event::Capabilities {
            capabilities: wayland_client::WEnum::Value(capabilities),
        } = event
        else {
            return;
        };
        let has_pointer = capabilities.contains(wl_seat::Capability::Pointer);
        match (has_pointer, app.pointer.is_some()) {
            (true, false) => app.pointer = Some(seat.get_pointer(queue, ())),
            (false, true) => {
                if let Some(pointer) = app.pointer.take() {
                    pointer.release();
                }
                app.pointer_moved(None);
            }
            _ => {}
        }
    }
}

impl Dispatch<WlPointer, ()> for App {
    fn event(
        app: &mut App,
        _: &WlPointer,
        event: wl_pointer::Event,
        _: &(),
        _: &Connection,
        _: &QueueHandle<App>,
    ) {
        match event {
            wl_pointer::Event::Enter {
                surface_x,
                surface_y,
                ..
            }
            | wl_pointer::Event::Motion {
                surface_x,
                surface_y,
                ..
            } => app.pointer_moved(Some((surface_x, surface_y))),
            wl_pointer::Event::Leave { .. } => app.pointer_moved(None),
            wl_pointer::Event::Button {
                state: wayland_client::WEnum::Value(wl_pointer::ButtonState::Pressed),
                ..
            } => app.pointer_pressed(),
            _ => {}
        }
    }
}

/// Regions are created only to be handed straight to `set_input_region`, so
/// they never have anything to say.
impl Dispatch<wl_region::WlRegion, ()> for App {
    fn event(
        _: &mut App,
        _: &wl_region::WlRegion,
        _: wl_region::Event,
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

#[cfg(test)]
mod tests {
    use super::Wanted;

    /// Walks a run of focus changes and requests, and reports what is on
    /// screen after each. `true` in `focus` means a window the rule names.
    fn run(start: Wanted, steps: &[(Option<Wanted>, bool)]) -> Vec<bool> {
        let (mut wanted, mut was_allowed) = (start, false);
        steps
            .iter()
            .map(|(asked, allowed)| {
                if let Some(asked) = *asked {
                    wanted = asked;
                }
                wanted = wanted.settle(*allowed, was_allowed);
                was_allowed = *allowed;
                wanted.shows(*allowed)
            })
            .collect()
    }

    /// Without a focus rule every window counts, so nothing ever *becomes*
    /// allowed and an override stands until the next one.
    #[test]
    fn with_no_focus_rule_show_and_hide_simply_stick() {
        assert_eq!(
            run(Wanted::Follow, &[
                (None, true),
                (Some(Wanted::Hidden), true),
                (None, true),
                (None, true),
                (Some(Wanted::Shown), true),
                (None, true),
            ]),
            [true, false, false, false, true, true]
        );
    }

    /// The everyday case: the window follows the game, and asking for it from
    /// outside lasts only until the game is next in front.
    #[test]
    fn an_override_is_spent_by_coming_back_to_the_window() {
        assert_eq!(
            run(Wanted::Follow, &[
                (None, true),                  // game in front
                (None, false),                 // alt-tabbed away
                (Some(Wanted::Shown), false),  // `show` from the terminal
                (None, false),                 // still away, still up
                (None, true),                  // back to the game: spent
                (None, false),                 // away again: follows the rule
            ]),
            [true, false, true, true, true, false]
        );
    }

    /// Hiding it while the game is in front lasts until the next time the
    /// game comes back, not for ever.
    #[test]
    fn hiding_lasts_until_the_window_is_next_focused() {
        assert_eq!(
            run(Wanted::Follow, &[
                (None, true),
                (Some(Wanted::Hidden), true),  // in the way; put it down
                (None, true),                  // stays down
                (None, false),                 // alt-tabbed away
                (None, true),                  // back again: spent
            ]),
            [true, false, false, false, true]
        );
    }

    /// `start-hidden` is exactly `hide` at startup, and behaves like it.
    #[test]
    fn starting_hidden_is_the_same_as_hiding() {
        assert_eq!(
            run(Wanted::Hidden, &[(None, false), (None, true), (None, false)]),
            [false, true, false]
        );
    }
}
