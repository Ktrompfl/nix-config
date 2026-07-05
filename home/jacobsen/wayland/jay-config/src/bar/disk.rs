use std::{ffi::CString, mem::MaybeUninit, time::Duration};

use super::schedule::repeat;

const PATH: &str = "/persist/";

fn usage(path: &str) -> String {
    let Ok(cpath) = CString::new(path) else {
        return String::new();
    };
    let mut stat = MaybeUninit::uninit();
    // SAFETY: `cpath` is a valid, NUL-terminated string and `stat` is a
    // valid, appropriately sized and aligned out-pointer for statvfs to fill in.
    let ret = unsafe { libc::statvfs(cpath.as_ptr(), stat.as_mut_ptr()) };
    if ret != 0 {
        return String::new();
    }
    // SAFETY: statvfs returned success, so `stat` was fully initialized.
    let stat = unsafe { stat.assume_init() };
    let total = stat.f_blocks as u64 * stat.f_frsize as u64;
    let free = stat.f_bavail as u64 * stat.f_frsize as u64;
    if total == 0 {
        return String::new();
    }
    let used_percent = (total - free) as f64 / total as f64 * 100.0;
    format!("\u{f02ca} {used_percent:.0}%")
}

pub fn run(on_update: impl Fn(String) + 'static) {
    repeat("bar-disk", Duration::from_secs(30), move || on_update(usage(PATH)));
}
