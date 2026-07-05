use jay_config::{
    exec::Command,
    set_show_bar,
    status::{MessageFormat, set_i3bar_separator, set_status_command},
    theme::{BarPosition, set_bar_position},
};

// The rest of the bar (workspaces, tray via wl-tray-bridge, clock) is
// handled natively by jay; i3status-rust only fills in the status text.
// See ../../programs/i3status-rust.nix for the actual block configuration.
pub fn setup() {
    set_show_bar(true);
    set_bar_position(BarPosition::Top);

    set_i3bar_separator(" ");
    set_status_command(MessageFormat::I3Bar, Command::new("i3status-rs").arg("config-jay.toml"));
}
