use std::{
    fs::File,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};

use jay_config::{
    exec::Command,
    set_show_bar,
    status::{MessageFormat, set_i3bar_separator, set_status_command},
};

/// The D-Bus name of the running i3status-rust.
static DBUS_NAME: Mutex<String> = Mutex::new(String::new());

/// Sets the text of one of i3status-rust's `custom_dbus` blocks.
fn set_text(path: &str, text: &str) {
    let name = DBUS_NAME.lock().unwrap().clone();
    let mut busctl = Command::new("busctl");
    for arg in [
        "--user",
        "call",
        name.as_str(),
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

pub fn set_idle_inhibitor(active: bool) {
    set_text("/idle_inhibitor", if active { "󰅶" } else { "󰾪" });
}

pub fn setup() {
    set_show_bar(true);
    set_i3bar_separator("");
    // a fresh name for every start: the previous i3status-rust keeps its name
    // until it notices on its next write that the bar is gone
    let suffix = format!(
        "jay{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos()
    );
    *DBUS_NAME.lock().unwrap() = format!("rs.i3status.{suffix}");
    set_status_command(
        MessageFormat::I3Bar,
        Command::new("i3status-rs")
            .arg("config-jay.toml")
            .env("I3RS_DBUS_NAME", &suffix),
    );

    // reset after a reload
    set_mode(None);
    set_idle_inhibitor(false);
}
