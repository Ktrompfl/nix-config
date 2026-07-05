use std::{fs, time::Duration};

use super::schedule::repeat;

struct Sample {
    idle: u64,
    total: u64,
}

fn read_sample() -> Option<Sample> {
    let content = fs::read_to_string("/proc/stat").ok()?;
    let line = content.lines().next()?;
    let nums: Vec<u64> = line.split_whitespace().skip(1).filter_map(|s| s.parse().ok()).collect();
    if nums.len() < 4 {
        return None;
    }
    // idle + iowait
    let idle = nums[3] + nums.get(4).copied().unwrap_or(0);
    let total = nums.iter().sum();
    Some(Sample { idle, total })
}

fn usage(prev: &mut Option<Sample>) -> String {
    let Some(sample) = read_sample() else {
        return String::new();
    };
    let text = match prev {
        Some(p) => {
            let idle_delta = sample.idle.saturating_sub(p.idle) as f64;
            let total_delta = sample.total.saturating_sub(p.total) as f64;
            let usage = if total_delta > 0.0 { (1.0 - idle_delta / total_delta) * 100.0 } else { 0.0 };
            format!("\u{f0f86} {usage:.0}%")
        }
        None => String::new(),
    };
    *prev = Some(sample);
    text
}

pub fn run(on_update: impl Fn(String) + 'static) {
    let mut prev = None;
    repeat("bar-cpu", Duration::from_secs(2), move || on_update(usage(&mut prev)));
}
