use std::{fs, time::Duration};

use super::{
    schedule::repeat,
    sysfs::{icon_index, read_trim},
};

const ICONS: [&str; 15] = [
    "\u{e3d5}", "\u{e3d4}", "\u{e3d3}", "\u{e3d2}", "\u{e3d1}", "\u{e3d0}", "\u{e3cf}", "\u{e3ce}", "\u{e3cd}",
    "\u{e3cc}", "\u{e3cb}", "\u{e3ca}", "\u{e3c9}", "\u{e3c8}", "\u{e3e3}",
];

fn status() -> String {
    let Ok(mut entries) = fs::read_dir("/sys/class/backlight") else {
        return String::new();
    };
    let Some(dir) = entries.next().and_then(Result::ok).map(|e| e.path()) else {
        return String::new();
    };
    let Some(brightness) = read_trim(dir.join("brightness")).and_then(|s| s.parse::<u32>().ok()) else {
        return String::new();
    };
    let Some(max) = read_trim(dir.join("max_brightness")).and_then(|s| s.parse::<u32>().ok()) else {
        return String::new();
    };
    if max == 0 {
        return String::new();
    }
    let percent = brightness * 100 / max;
    ICONS[icon_index(percent, ICONS.len())].to_string()
}

pub fn run(on_update: impl Fn(String) + 'static) {
    repeat("bar-backlight", Duration::from_secs(5), move || on_update(status()));
}
