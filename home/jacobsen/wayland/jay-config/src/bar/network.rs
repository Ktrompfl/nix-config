use serde_json::Value;

use super::{i3status, sysfs::icon_index};

const WIFI_ICONS: [&str; 5] = ["\u{f092f}", "\u{f091f}", "\u{f0922}", "\u{f0925}", "\u{f0928}"];
const WIRED_ICON: &str = "\u{f0318}";
const DOWN_ICON: &str = "\u{f0319}";

/// `status` is a literal tag (`config-jay.toml` picks it per which of
/// `format`/`inactive_format`/`missing_format` i3status-rs rendered); wifi
/// vs. wired is then just whether `signal_strength` is present.
fn format(data: &Value) -> String {
    match i3status::text(data, "status") {
        Some("up") => match i3status::number(data, "signal_strength") {
            Some(percent) => {
                let percent = percent.clamp(0.0, 100.0) as u32;
                WIFI_ICONS[icon_index(percent, WIFI_ICONS.len())].to_string()
            }
            None => WIRED_ICON.to_string(),
        },
        Some("down" | "missing") => DOWN_ICON.to_string(),
        _ => String::new(),
    }
}

pub fn run(on_update: impl Fn(String) + 'static) {
    i3status::subscribe(6, move |data| on_update(format(data)));
}
