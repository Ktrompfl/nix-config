use std::{rc::Rc, time::Duration};

use jay_config::exec::Command;

use super::{exec::capture, schedule::repeat};

const VOLUME_ICONS: [&str; 3] = ["\u{f026}", "\u{f027}", "\u{f057e}"];
const MUTED_ICON: &str = "\u{f466}";

fn poll(on_result: impl FnOnce(String) + 'static) {
    capture(Command::new("wpctl").arg("get-volume").arg("@DEFAULT_AUDIO_SINK@"), move |output| {
        on_result(format(&output));
    });
}

fn format(output: &str) -> String {
    let Some(line) = output.lines().next() else {
        return String::new();
    };
    if line.contains("[MUTED]") {
        return MUTED_ICON.to_string();
    }
    let Some(volume) = line.split_whitespace().nth(1).and_then(|s| s.parse::<f64>().ok()) else {
        return String::new();
    };
    let percent = (volume * 100.0).round() as i32;
    let icon = match percent {
        ..34 => VOLUME_ICONS[0],
        34..67 => VOLUME_ICONS[1],
        _ => VOLUME_ICONS[2],
    };
    format!("{icon} {percent}%")
}

pub fn run(on_update: impl Fn(String) + 'static) {
    let on_update = Rc::new(on_update);
    repeat("bar-volume", Duration::from_secs(2), move || {
        let on_update = on_update.clone();
        poll(move |text| on_update(text));
    });
}
