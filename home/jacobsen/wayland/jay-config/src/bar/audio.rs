use serde_json::Value;

use super::i3status;

const VOLUME_ICONS: [&str; 3] = ["\u{f026}", "\u{f027}", "\u{f057e}"];
const MUTED_ICON: &str = "\u{f466}";

/// The sound block's `volume` field is only absent (`null`) when muted -
/// `show_volume_when_muted` is off - so its absence doubles as the mute flag.
fn format(data: &Value) -> String {
    match i3status::number(data, "volume") {
        Some(percent) => {
            let percent = percent.round() as i32;
            let icon = match percent {
                ..34 => VOLUME_ICONS[0],
                34..67 => VOLUME_ICONS[1],
                _ => VOLUME_ICONS[2],
            };
            format!("{icon} {percent}%")
        }
        None => MUTED_ICON.to_string(),
    }
}

pub fn run(on_update: impl Fn(String) + 'static) {
    i3status::subscribe(3, move |data| on_update(format(data)));
}
