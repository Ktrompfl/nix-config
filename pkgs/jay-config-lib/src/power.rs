use std::fs;

use jay_config::exec::Command;

/// Whether the kernel reports suspend-to-disk support. False when there's no
/// swap (or swap file) to hibernate into, e.g. `disk` absent from
/// `/sys/power/state`.
fn hibernate_available() -> bool {
    fs::read_to_string("/sys/power/state")
        .map(|states| states.split_whitespace().any(|s| s == "disk"))
        .unwrap_or(false)
}

/// Suspends, falling back to suspend-then-hibernate when the system
/// supports hibernation.
pub fn suspend() {
    let action = if hibernate_available() {
        "suspend-then-hibernate"
    } else {
        "suspend"
    };
    Command::new("systemctl").arg(action).spawn();
}
