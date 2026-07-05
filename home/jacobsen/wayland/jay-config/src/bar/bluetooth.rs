use jay_config::exec::Command;

use super::exec::capture;

pub fn poll(on_result: impl FnOnce(String) + 'static) {
    capture(
        Command::new("sh").arg("-c").arg("bluetoothctl show; echo ---; bluetoothctl devices Connected"),
        move |output| on_result(format_bluetooth(&output)),
    );
}

fn format_bluetooth(output: &str) -> String {
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
