use std::cell::RefCell;

use jay_config::{
    set_show_bar,
    status::set_status,
    theme::{BarPosition, set_bar_position},
};

mod audio;
mod backlight;
mod battery;
mod bluetooth;
mod clock;
mod cpu;
mod disk;
mod exec;
mod i3status;
mod memory;
mod network;
mod notifications;
mod schedule;
mod sysfs;

// The rest of the bar (workspaces and the tray, via wl-tray-bridge) is
// handled natively by jay; this module renders the status text itself, in
// pango markup, instead of delegating to an external status command.
//
// Each segment module below owns its full lifecycle (scheduling, data
// capture, and formatting) behind a `run(on_update)` entry point; this file
// is only responsible for aggregating their output into the final status
// text and handing it to jay.

// TODO: placeholder codepoints, replace with the real icons.
const IDLE_INHIBITOR_ON_ICON: &str = "\u{f06e}";
const IDLE_INHIBITOR_OFF_ICON: &str = "\u{f070}";

#[derive(Default)]
struct Segments {
    mode: String,
    cpu: String,
    memory: String,
    disk: String,
    network: String,
    bluetooth: String,
    backlight: String,
    battery: String,
    volume: String,
    notifications: String,
    idle_inhibitor: String,
    clock: String,
}

impl Segments {
    fn render(&self) -> String {
        [
            &self.mode,
            &self.cpu,
            &self.memory,
            &self.disk,
            &self.network,
            &self.bluetooth,
            &self.backlight,
            &self.battery,
            &self.volume,
            &self.notifications,
            &self.idle_inhibitor,
            &self.clock,
        ]
        .into_iter()
        .filter(|segment| !segment.is_empty())
        .map(String::as_str)
        .collect::<Vec<_>>()
        .join("  ")
    }
}

thread_local! {
    static SEGMENTS: RefCell<Segments> = RefCell::new(Segments::default());
}

/// Mutates the segment state without rendering. i3status-backed segments
/// rely on this: a single input line can update several of them (see
/// `bar/i3status.rs`'s `dispatch`), and they should only render once, after
/// all of them have been applied - rendering here too would put one
/// `set_status` call per block back on the hot path.
fn update(f: impl FnOnce(&mut Segments)) {
    SEGMENTS.with(|segments| f(&mut segments.borrow_mut()));
}

fn render() {
    SEGMENTS.with(|segments| set_status(&segments.borrow().render()));
}

/// Called from `crate::modes` to reflect the active mode, mirroring the
/// waybar "custom/mode" module. Unlike the other segments this is pushed
/// reactively from keybindings rather than polled, so it has no `run`
/// function of its own, and renders immediately since it isn't part of an
/// i3status batch.
pub fn set_mode(mode: Option<&str>) {
    let mode = mode.unwrap_or("normal");
    update(|s| s.mode = mode.to_uppercase());
    render();
}

/// Called from `crate::shortcuts` when the idle inhibitor is toggled;
/// reactively pushed for the same reason as `set_mode` above.
pub fn set_idle_inhibitor(active: bool) {
    let icon = if active { IDLE_INHIBITOR_ON_ICON } else { IDLE_INHIBITOR_OFF_ICON };
    update(|s| s.idle_inhibitor = icon.to_string());
    render();
}

pub fn setup() {
    set_show_bar(true);
    set_bar_position(BarPosition::Bottom);

    set_mode(None);
    set_idle_inhibitor(false);

    // Feeds every segment below except mode/clock; see bar/i3status.rs.
    // `on_settled` collapses however many blocks a single i3status-rs line
    // touched into the one render it should produce.
    i3status::on_settled(render);
    i3status::run();

    cpu::run(|text| update(|s| s.cpu = text));
    memory::run(|text| update(|s| s.memory = text));
    disk::run(|text| update(|s| s.disk = text));
    network::run(|text| update(|s| s.network = text));
    backlight::run(|text| update(|s| s.backlight = text));
    battery::run(|text| update(|s| s.battery = text));
    bluetooth::run(|text| update(|s| s.bluetooth = text));
    audio::run(|text| update(|s| s.volume = text));
    notifications::run(|text| update(|s| s.notifications = text));
    clock::run(|text| {
        update(|s| s.clock = text);
        render();
    });
}
