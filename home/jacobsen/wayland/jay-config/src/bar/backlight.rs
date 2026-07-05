use std::path::{Path, PathBuf};

use inotify::{Inotify, WatchMask};
use jay_config::{io::Async, tasks::spawn};

use super::sysfs::{icon_index, read_trim};

const ICONS: [&str; 15] = [
    "\u{e3d5}", "\u{e3d4}", "\u{e3d3}", "\u{e3d2}", "\u{e3d1}", "\u{e3d0}", "\u{e3cf}", "\u{e3ce}", "\u{e3cd}",
    "\u{e3cc}", "\u{e3cb}", "\u{e3ca}", "\u{e3c9}", "\u{e3c8}", "\u{e3e3}",
];

fn find_device_dir() -> Option<PathBuf> {
    std::fs::read_dir("/sys/class/backlight").ok()?.next().and_then(Result::ok).map(|e| e.path())
}

fn status(dir: &Path) -> String {
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

/// Watches the backlight device's sysfs file for changes via inotify instead
/// of polling, so hotkey-driven brightness changes (which the kernel applies
/// by writing directly to this file) show up instantly.
pub fn run(on_update: impl Fn(String) + 'static) {
    let Some(dir) = find_device_dir() else {
        on_update(String::new());
        return;
    };

    on_update(status(&dir));

    let Ok(inotify) = Inotify::init() else {
        return;
    };
    if inotify.watches().add(dir.join("brightness"), WatchMask::MODIFY).is_err() {
        return;
    }
    let Ok(mut inotify) = Async::new(inotify) else {
        return;
    };

    spawn(async move {
        let mut buffer = [0u8; 1024];
        loop {
            if inotify.readable().await.is_err() {
                return;
            }
            if inotify.as_mut().read_events(&mut buffer).is_err() {
                continue;
            }
            on_update(status(&dir));
        }
    });
}
