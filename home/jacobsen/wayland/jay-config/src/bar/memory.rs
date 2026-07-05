use std::{fs, time::Duration};

use super::schedule::repeat;

fn meminfo_kb(content: &str, key: &str) -> Option<u64> {
    content.lines().find_map(|line| line.strip_prefix(key)?.trim().split_whitespace().next()?.parse().ok())
}

fn usage() -> String {
    let Ok(content) = fs::read_to_string("/proc/meminfo") else {
        return String::new();
    };
    let total = meminfo_kb(&content, "MemTotal:");
    let available = meminfo_kb(&content, "MemAvailable:");
    match (total, available) {
        (Some(t), Some(a)) if t > 0 => {
            let percent = (t - a) as f64 / t as f64 * 100.0;
            format!("\u{f035b} {percent:.0}%")
        }
        _ => String::new(),
    }
}

pub fn run(on_update: impl Fn(String) + 'static) {
    repeat("bar-memory", Duration::from_secs(5), move || on_update(usage()));
}
