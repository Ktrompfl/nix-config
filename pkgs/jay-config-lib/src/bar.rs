use std::fs::File;

use jay_config::{exec::Command, set_show_bar};

/// Sets the text of one of the bar's i3status-rust `custom_dbus` blocks.
fn set_text(path: &str, text: &str) {
    let mut busctl = Command::new("busctl");
    for arg in [
        "--user",
        "call",
        // fixed by I3RS_DBUS_NAME in the i3bar-wl config
        "rs.i3status.bar",
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
    set_text("/mode", &mode.unwrap_or_default().to_uppercase());
}

pub fn setup() {
    // i3bar-wl, a layer-shell client shared with getaway, is the bar
    set_show_bar(false);

    // reset after a reload
    set_mode(None);
}
