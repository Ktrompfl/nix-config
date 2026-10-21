use std::fs::File;

use jay_config::{
    exec::Command,
    set_show_bar,
    status::{MessageFormat, set_i3bar_separator, set_status_command},
};

/// Sets the text of one of i3status-rust's `custom_dbus` blocks.
fn set_text(path: &str, text: &str) {
    let mut busctl = Command::new("busctl");
    for arg in [
        "--user",
        "call",
        "rs.i3status",
        path,
        "rs.i3status.custom",
        "SetText",
        "ss",
        text,
        "",
    ] {
        busctl.arg(arg);
    }
    // fails while i3status-rust is not on the bus yet
    if let Ok(null) = File::options().write(true).open("/dev/null")
        && let Ok(null2) = null.try_clone()
    {
        busctl.stdout(null).stderr(null2);
    }
    busctl.spawn();
}

pub fn set_mode(mode: Option<&str>) {
    set_text("/mode", &mode.unwrap_or("normal").to_uppercase());
}

pub fn set_idle_inhibitor(active: bool) {
    set_text("/idle_inhibitor", if active { "󰅶" } else { "󰾪" });
}

pub fn setup() {
    set_show_bar(true);
    set_i3bar_separator("");
    set_status_command(
        MessageFormat::I3Bar,
        Command::new("i3status-rs").arg("config-jay.toml"),
    );

    // reset after a reload
    set_mode(None);
    set_idle_inhibitor(false);
}
