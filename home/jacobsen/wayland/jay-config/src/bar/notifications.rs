use jay_config::exec::Command;

use super::exec::capture_lines;
use crate::generated::BASE08;

fn format(line: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(line) else {
        return String::new();
    };
    let text = value.get("text").and_then(|v| v.as_str()).unwrap_or("");
    if text.is_empty() || text == "0" {
        return String::new();
    }
    format!("<span foreground=\"#{BASE08}\">\u{f0a2}<sup>\u{f444}</sup></span>")
}

/// `swaync-client -swb` subscribes and stays running, printing a fresh JSON
/// line whenever notification state changes (including its current state
/// right away), so this reacts to that stream instead of polling.
pub fn run(on_update: impl Fn(String) + 'static) {
    capture_lines(Command::new("swaync-client").arg("-swb"), move |line| on_update(format(&line)));
}
