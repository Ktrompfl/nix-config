use jay_config::{
    exec::Command,
    set_show_bar,
    status::{MessageFormat, set_i3bar_separator, set_status_command},
    theme::{BarPosition, set_bar_position},
};

// Everything that is shown in the bar is rendered by `i3status-rs`, which
// decides icons, thresholds, severity colors, and which blocks to hide (see
// home/jacobsen/programs/i3status-rust.nix). Jay only concatenates the
// blocks it prints, which is why the separator is empty: every block brings
// its own padding.
//
// The two segments that reflect compositor state - the active input mode and
// the idle inhibitor - cannot be observed from the outside, so they are
// `custom_dbus` blocks that this module pushes into whenever the state
// changes, exactly like the toml configuration does.

/// The object paths of the two `custom_dbus` blocks.
const MODE_PATH: &str = "/mode";
const IDLE_INHIBITOR_PATH: &str = "/idle_inhibitor";

const IDLE_INHIBITOR_ON_ICON: &str = "";
const IDLE_INHIBITOR_OFF_ICON: &str = "";

fn set_block_text(path: &str, text: &str) {
    Command::new("busctl")
        .arg("--user")
        .arg("call")
        .arg("rs.i3status")
        .arg(path)
        .arg("rs.i3status.custom")
        .arg("SetText")
        .arg("ss")
        .arg(text)
        .arg("")
        .spawn();
}

/// Called from `crate::shortcuts` to reflect the active mode. Only the mode
/// that was entered last is ever shown, which is all the shortcuts push.
pub fn set_mode(mode: Option<&str>) {
    set_block_text(MODE_PATH, &mode.unwrap_or("normal").to_uppercase());
}

/// Called from `crate::shortcuts` when the idle inhibitor is toggled.
pub fn set_idle_inhibitor(active: bool) {
    let icon = if active {
        IDLE_INHIBITOR_ON_ICON
    } else {
        IDLE_INHIBITOR_OFF_ICON
    };
    set_block_text(IDLE_INHIBITOR_PATH, icon);
}

/// A `custom_dbus` block stays invisible until something has pushed a value
/// into it, so both of them are seeded once the bar is up. `i3status-rs` only
/// claims the bus name a moment after it has been spawned, hence the retry.
fn seed() {
    let set_text = |path: &str, text: &str| {
        format!(
            r#"busctl --user call rs.i3status {path} rs.i3status.custom SetText ss "{text}" "" >/dev/null 2>&1"#
        )
    };
    let script = format!(
        "for _ in $(seq 100); do
             if {mode} && {idle_inhibitor}; then exit 0; fi
             sleep 0.1
         done",
        mode = set_text(MODE_PATH, "NORMAL"),
        idle_inhibitor = set_text(IDLE_INHIBITOR_PATH, IDLE_INHIBITOR_OFF_ICON),
    );
    Command::new("sh").arg("-c").arg(&script).spawn();
}

pub fn setup() {
    set_show_bar(true);
    set_bar_position(BarPosition::Bottom);

    // every block brings its own padding
    set_i3bar_separator("");
    set_status_command(
        MessageFormat::I3Bar,
        Command::new("i3status-rs").arg("config-jay.toml"),
    );

    seed();
}
