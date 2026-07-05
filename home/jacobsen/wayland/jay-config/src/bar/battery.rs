use std::{fs, path::PathBuf, time::Duration};

use super::{
    schedule::repeat,
    sysfs::{icon_index, read_trim},
};
use crate::generated::{BASE08, BASE09};

const ICONS: [&str; 11] = [
    "\u{f008e}", "\u{f007a}", "\u{f007b}", "\u{f007c}", "\u{f007d}", "\u{f007e}", "\u{f007f}", "\u{f0080}",
    "\u{f0081}", "\u{f0082}", "\u{f0079}",
];

fn find_dir() -> Option<PathBuf> {
    let entries = fs::read_dir("/sys/class/power_supply").ok()?;
    entries.flatten().map(|e| e.path()).find(|path| read_trim(path.join("type")).as_deref() == Some("Battery"))
}

fn status() -> String {
    let Some(dir) = find_dir() else {
        return String::new();
    };
    let Some(capacity) = read_trim(dir.join("capacity")).and_then(|s| s.parse::<u32>().ok()) else {
        return String::new();
    };
    let status = read_trim(dir.join("status")).unwrap_or_default();

    let body = match status.as_str() {
        "Charging" => format!("\u{f0e7} {capacity}%"),
        "Full" | "Not charging" => format!("\u{f1616} {capacity}%"),
        _ => format!("{} {capacity}%", ICONS[icon_index(capacity, ICONS.len())]),
    };

    if capacity <= 15 {
        format!("<span foreground=\"#{BASE08}\">{body}</span>")
    } else if capacity <= 30 {
        format!("<span foreground=\"#{BASE09}\">{body}</span>")
    } else {
        body
    }
}

pub fn run(on_update: impl Fn(String) + 'static) {
    repeat("bar-battery", Duration::from_secs(30), move || on_update(status()));
}
