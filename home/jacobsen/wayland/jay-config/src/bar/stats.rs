use std::fs;

pub struct CpuSample {
    idle: u64,
    total: u64,
}

fn read_cpu_sample() -> Option<CpuSample> {
    let content = fs::read_to_string("/proc/stat").ok()?;
    let line = content.lines().next()?;
    let nums: Vec<u64> = line.split_whitespace().skip(1).filter_map(|s| s.parse().ok()).collect();
    if nums.len() < 4 {
        return None;
    }
    // idle + iowait
    let idle = nums[3] + nums.get(4).copied().unwrap_or(0);
    let total = nums.iter().sum();
    Some(CpuSample { idle, total })
}

/// CPU usage since the previous call. `prev` should be reused across calls.
pub fn cpu_usage(prev: &mut Option<CpuSample>) -> String {
    let Some(sample) = read_cpu_sample() else {
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

fn meminfo_kb(content: &str, key: &str) -> Option<u64> {
    content.lines().find_map(|line| {
        line.strip_prefix(key)?.trim().split_whitespace().next()?.parse().ok()
    })
}

pub fn memory_usage() -> String {
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
