use serde_json::Value;

use super::{i3status, sysfs::icon_index};

const ICONS: [&str; 15] = [
    "\u{e3d5}", "\u{e3d4}", "\u{e3d3}", "\u{e3d2}", "\u{e3d1}", "\u{e3d0}", "\u{e3cf}", "\u{e3ce}", "\u{e3cd}",
    "\u{e3cc}", "\u{e3cb}", "\u{e3ca}", "\u{e3c9}", "\u{e3c8}", "\u{e3e3}",
];

/// `brightness` is only absent (`null`) via `missing_format` when there's no
/// backlight device.
fn format(data: &Value) -> String {
    let Some(percent) = i3status::number(data, "brightness") else {
        return String::new();
    };
    let percent = percent.clamp(0.0, 100.0) as u32;
    ICONS[icon_index(percent, ICONS.len())].to_string()
}

pub fn run(on_update: impl Fn(String) + 'static) {
    i3status::subscribe(4, move |data| on_update(format(data)));
}
