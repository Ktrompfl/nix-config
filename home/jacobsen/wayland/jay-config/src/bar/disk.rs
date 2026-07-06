use serde_json::Value;

use super::i3status;
use crate::generated::{BASE08, BASE09};

const ICON: &str = "\u{f02ca}";

const WARNING: f64 = 80.0;
const CRITICAL: f64 = 90.0;

fn severity(percent: f64) -> Option<&'static str> {
    if percent >= CRITICAL {
        Some(BASE08)
    } else if percent >= WARNING {
        Some(BASE09)
    } else {
        None
    }
}

fn format(data: &Value) -> String {
    let Some(percent) = i3status::number(data, "percentage") else {
        return String::new();
    };
    let body = format!("{ICON} {percent:.0}%");
    match severity(percent) {
        Some(color) => format!("<span foreground=\"#{color}\">{body}</span>"),
        None => body,
    }
}

pub fn run(on_update: impl Fn(String) + 'static) {
    i3status::subscribe(2, move |data| on_update(format(data)));
}
