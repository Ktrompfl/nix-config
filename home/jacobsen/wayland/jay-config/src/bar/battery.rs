use serde_json::Value;

use super::{i3status, sysfs::icon_index};
use crate::generated::{BASE08, BASE09};

const ICONS: [&str; 11] = [
    "\u{f008e}", "\u{f007a}", "\u{f007b}", "\u{f007c}", "\u{f007d}", "\u{f007e}", "\u{f007f}", "\u{f0080}",
    "\u{f0081}", "\u{f0082}", "\u{f0079}",
];

const WARNING: f64 = 30.0;
const CRITICAL: f64 = 15.0;

fn severity(capacity: f64) -> Option<&'static str> {
    if capacity <= CRITICAL {
        Some(BASE08)
    } else if capacity <= WARNING {
        Some(BASE09)
    } else {
        None
    }
}

/// `status` is a literal tag (`config-jay.toml` picks it per which of
/// `format`/`charging_format`/`full_format`/`empty_format`/
/// `not_charging_format`/`missing_format` i3status-rs rendered), since
/// there's no separate placeholder for battery state.
fn format(data: &Value) -> String {
    let status = i3status::text(data, "status").unwrap_or("missing");
    if status == "missing" {
        return String::new();
    }
    let Some(capacity) = i3status::number(data, "percentage") else {
        return String::new();
    };

    let body = match status {
        "charging" => format!("\u{f0e7} {capacity:.0}%"),
        "full" | "not_charging" => format!("\u{f1616} {capacity:.0}%"),
        _ => {
            let icon = ICONS[icon_index(capacity.clamp(0.0, 100.0) as u32, ICONS.len())];
            format!("{icon} {capacity:.0}%")
        }
    };

    match severity(capacity) {
        Some(color) => format!("<span foreground=\"#{color}\">{body}</span>"),
        None => body,
    }
}

pub fn run(on_update: impl Fn(String) + 'static) {
    i3status::subscribe(5, move |data| on_update(format(data)));
}
