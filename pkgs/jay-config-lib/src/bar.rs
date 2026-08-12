//! Everything that is shown in the bar is rendered by i3status-rust, which
//! decides icons, thresholds, severity colors, and which blocks to hide; see
//! home/jacobsen/programs/i3status-rust.nix. Jay only concatenates the blocks
//! it prints, which is why the separator is empty: every block brings its own
//! padding.

use jay_config::{
    exec::Command,
    set_show_bar,
    status::{MessageFormat, set_i3bar_separator, set_status_command},
};

use crate::exec;

/// Shows the active input mode, `None` being the normal mode.
pub fn set_mode(mode: Option<&str>) {
    exec("jay-bar", &["mode", mode.unwrap_or("normal")]);
}

pub fn set_idle_inhibitor(active: bool) {
    exec(
        "jay-bar",
        &["idle-inhibitor", if active { "on" } else { "off" }],
    );
}

pub fn setup() {
    set_show_bar(true);
    set_i3bar_separator("");
    set_status_command(
        MessageFormat::I3Bar,
        Command::new("i3status-rs").arg("config-jay.toml"),
    );

    exec("jay-bar", &["init"]);
}
