use serde_json::Value;

use super::i3status;
use crate::theme;

const BELL_ICON: &str = "\u{f0a2}";
const DND_ICON: &str = "\u{f1f7}";
const BADGE_ICON: &str = "\u{f444}";

/// `notification_count` is only present (never `0`) when there are
/// pending notifications, independent of `paused` (do-not-disturb), so the
/// two combine into the same four states the old swaync-client-based
/// version showed.
fn format(data: &Value) -> String {
    let icon = if i3status::text(data, "paused") == Some("true") { DND_ICON } else { BELL_ICON };
    match i3status::number(data, "notification_count") {
        Some(count) => {
            let base08 = theme::base08();
            format!("{icon}<span foreground=\"#{base08}\"><sup>{BADGE_ICON}</sup></span> {count:.0}")
        }
        None => icon.to_string(),
    }
}

pub fn run(on_update: impl Fn(String) + 'static) {
    i3status::subscribe(7, move |data| on_update(format(data)));
}
