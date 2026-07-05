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

fn update(f: impl FnOnce(&mut Segments)) {
    SEGMENTS.with(|segments| {
        let mut segments = segments.borrow_mut();
        f(&mut segments);
        set_status(&segments.render());
    });
}

/// Called from `crate::modes` to reflect the active mode, mirroring the
/// waybar "custom/mode" module. Unlike the other segments this is pushed
/// reactively from keybindings rather than polled, so it has no `run`
/// function of its own.
pub fn set_mode(mode: Option<&str>) {
    update(|s| s.mode = mode.map(str::to_uppercase).unwrap_or_default());
}

pub fn setup() {
    set_show_bar(true);
    set_bar_position(BarPosition::Top);

    cpu::run(|text| update(|s| s.cpu = text));
    memory::run(|text| update(|s| s.memory = text));
    disk::run(|text| update(|s| s.disk = text));
    network::run(|text| update(|s| s.network = text));
    backlight::run(|text| update(|s| s.backlight = text));
    battery::run(|text| update(|s| s.battery = text));
    bluetooth::run(|text| update(|s| s.bluetooth = text));
    audio::run(|text| update(|s| s.volume = text));
    notifications::run(|text| update(|s| s.notifications = text));
    clock::run(|text| update(|s| s.clock = text));
}
