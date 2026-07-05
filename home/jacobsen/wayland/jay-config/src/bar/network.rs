use std::{fs, time::Duration};

use super::{
    schedule::repeat,
    sysfs::{icon_index, read_trim},
};

const WIFI_ICONS: [&str; 5] = ["\u{f092f}", "\u{f091f}", "\u{f0922}", "\u{f0925}", "\u{f0928}"];

fn wifi_signal_percent(ifname: &str) -> Option<u32> {
    let content = fs::read_to_string("/proc/net/wireless").ok()?;
    for line in content.lines().skip(2) {
        let mut parts = line.split_whitespace();
        let iface = parts.next()?.trim_end_matches(':');
        if iface != ifname {
            continue;
        }
        let _status = parts.next()?;
        let link: f64 = parts.next()?.parse().ok()?;
        // link quality is out of 70 (see /proc/net/wireless / iwconfig)
        return Some(((link / 70.0) * 100.0).clamp(0.0, 100.0) as u32);
    }
    None
}

fn status() -> String {
    let Ok(entries) = fs::read_dir("/sys/class/net") else {
        return String::new();
    };
    for entry in entries.flatten() {
        let name = entry.file_name().to_string_lossy().into_owned();
        if name == "lo" {
            continue;
        }
        let path = entry.path();
        if read_trim(path.join("operstate")).as_deref() != Some("up") {
            continue;
        }
        if path.join("wireless").is_dir() {
            let percent = wifi_signal_percent(&name).unwrap_or(0);
            return WIFI_ICONS[icon_index(percent, WIFI_ICONS.len())].to_string();
        }
        return "\u{f0318}".to_string();
    }
    "\u{f0319}".to_string()
}

pub fn run(on_update: impl Fn(String) + 'static) {
    repeat("bar-network", Duration::from_secs(5), move || on_update(status()));
}
