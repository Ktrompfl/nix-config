use serde_json::Value;

use super::i3status;
use crate::generated::{BASE08, BASE09, BASE0A};

const ICON: &str = "\u{f0f86}";

const INFO: f64 = 30.0;
const WARNING: f64 = 60.0;
const CRITICAL: f64 = 90.0;

fn severity(usage: f64) -> Option<&'static str> {
    if usage >= CRITICAL {
        Some(BASE08)
    } else if usage >= WARNING {
        Some(BASE09)
    } else if usage >= INFO {
        Some(BASE0A)
    } else {
        None
    }
}

fn format(data: &Value) -> String {
    let Some(usage) = i3status::number(data, "utilization") else {
        return String::new();
    };
    let body = format!("{ICON} {usage:.0}%");
    match severity(usage) {
        Some(color) => format!("<span foreground=\"#{color}\">{body}</span>"),
        None => body,
    }
}

pub fn run(on_update: impl Fn(String) + 'static) {
    i3status::subscribe(0, move |data| on_update(format(data)));
}
