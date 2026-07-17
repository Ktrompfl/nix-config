use serde_json::Value;

use super::i3status;

const CONNECTED_ICON: &str = "\u{f00b1}";
const UNAVAILABLE_ICON: &str = "\u{f00b3}";

/// `status` is a literal tag (`config-jay.toml` picks it per which of
/// `format`/`disconnected_format` i3status-rs rendered); `available`
/// further distinguishes "known to bluez but off" (nothing shown, like the
/// old adapter-powered-but-nothing-connected case) from truly unavailable.
fn format(data: &Value) -> String {
    match i3status::text(data, "status") {
        Some("connected") => match i3status::number(data, "percentage") {
            Some(percent) => format!("{CONNECTED_ICON} {percent:.0}%"),
            None => CONNECTED_ICON.to_string(),
        },
        Some("disconnected") if i3status::text(data, "available") == Some("true") => String::new(),
        _ => UNAVAILABLE_ICON.to_string(),
    }
}

pub fn run(on_update: impl Fn(String) + 'static) {
    i3status::subscribe(8, move |data| on_update(format(data)));
}
