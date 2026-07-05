use std::{rc::Rc, time::Duration};

use jay_config::exec::Command;

use super::{exec::capture, schedule::repeat};
use crate::generated::BASE08;

fn poll(on_result: impl FnOnce(String) + 'static) {
    capture(Command::new("swaync-client").arg("-swb"), move |output| {
        on_result(format(&output));
    });
}

fn format(output: &str) -> String {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(output) else {
        return String::new();
    };
    let text = value.get("text").and_then(|v| v.as_str()).unwrap_or("");
    if text.is_empty() || text == "0" {
        return String::new();
    }
    format!("<span foreground=\"#{BASE08}\">\u{f0a2}<sup>\u{f444}</sup></span>")
}

pub fn run(on_update: impl Fn(String) + 'static) {
    let on_update = Rc::new(on_update);
    repeat("bar-notifications", Duration::from_secs(5), move || {
        let on_update = on_update.clone();
        poll(move |text| on_update(text));
    });
}
