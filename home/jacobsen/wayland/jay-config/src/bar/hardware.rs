use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::generated::{BASE08, BASE09};

fn read_trim(path: impl AsRef<Path>) -> Option<String> {
    fs::read_to_string(path).ok().map(|s| s.trim().to_string())
}

const BATTERY_ICONS: [&str; 11] = [
    "\u{f008e}", "\u{f007a}", "\u{f007b}", "\u{f007c}", "\u{f007d}", "\u{f007e}", "\u{f007f}", "\u{f0080}",
    "\u{f0081}", "\u{f0082}", "\u{f0079}",
];

fn find_battery_dir() -> Option<PathBuf> {
    let entries = fs::read_dir("/sys/class/power_supply").ok()?;
    entries.flatten().map(|e| e.path()).find(|path| read_trim(path.join("type")).as_deref() == Some("Battery"))
}

fn icon_index(percent: u32, len: usize) -> usize {
    ((percent as usize) * (len - 1) / 100).min(len - 1)
}

pub fn battery() -> String {
    let Some(dir) = find_battery_dir() else {
        return String::new();
    };
    let Some(capacity) = read_trim(dir.join("capacity")).and_then(|s| s.parse::<u32>().ok()) else {
        return String::new();
    };
    let status = read_trim(dir.join("status")).unwrap_or_default();

    let body = match status.as_str() {
        "Charging" => format!("\u{f0e7} {capacity}%"),
        "Full" | "Not charging" => format!("\u{f1616} {capacity}%"),
        _ => format!("{} {capacity}%", BATTERY_ICONS[icon_index(capacity, BATTERY_ICONS.len())]),
    };

    if capacity <= 15 {
        format!("<span foreground=\"#{BASE08}\">{body}</span>")
    } else if capacity <= 30 {
        format!("<span foreground=\"#{BASE09}\">{body}</span>")
    } else {
        body
    }
}

const BACKLIGHT_ICONS: [&str; 15] = [
    "\u{e3d5}", "\u{e3d4}", "\u{e3d3}", "\u{e3d2}", "\u{e3d1}", "\u{e3d0}", "\u{e3cf}", "\u{e3ce}", "\u{e3cd}",
    "\u{e3cc}", "\u{e3cb}", "\u{e3ca}", "\u{e3c9}", "\u{e3c8}", "\u{e3e3}",
];

pub fn backlight() -> String {
    let Ok(mut entries) = fs::read_dir("/sys/class/backlight") else {
        return String::new();
    };
    let Some(dir) = entries.next().and_then(Result::ok).map(|e| e.path()) else {
        return String::new();
    };
    let Some(brightness) = read_trim(dir.join("brightness")).and_then(|s| s.parse::<u32>().ok()) else {
        return String::new();
    };
    let Some(max) = read_trim(dir.join("max_brightness")).and_then(|s| s.parse::<u32>().ok()) else {
        return String::new();
    };
    if max == 0 {
        return String::new();
    }
    let percent = brightness * 100 / max;
    BACKLIGHT_ICONS[icon_index(percent, BACKLIGHT_ICONS.len())].to_string()
}

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

pub fn network() -> String {
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
