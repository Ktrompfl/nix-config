use std::{rc::Rc, time::Duration};

use jay_config::exec::Command;

use super::{exec::capture, schedule::repeat};

fn poll(on_result: impl FnOnce(String) + 'static) {
    capture(
        Command::new("sh").arg("-c").arg("bluetoothctl show; echo ---; bluetoothctl devices Connected"),
        move |output| on_result(format(&output)),
    );
}

fn format(output: &str) -> String {
    let mut parts = output.splitn(2, "---");
    let show = parts.next().unwrap_or_default();
    let devices = parts.next().unwrap_or_default();

    let powered = show.lines().any(|line| line.trim() == "Powered: yes");
    if !powered {
        return "\u{f00b3}".to_string();
    }

    let connected = devices.lines().filter(|line| line.trim_start().starts_with("Device")).count();
    if connected > 0 { format!("\u{f00b1} {connected}") } else { String::new() }
}

pub fn run(on_update: impl Fn(String) + 'static) {
    let on_update = Rc::new(on_update);
    repeat("bar-bluetooth", Duration::from_secs(15), move || {
        let on_update = on_update.clone();
        poll(move |text| on_update(text));
    });
}
