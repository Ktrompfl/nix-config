use std::time::Duration;

use jay_config::{set_idle, set_idle_grace_period};

/// No idle timer of jay's own: swayidle (idle.service, shared with getaway)
/// decides when, and turns the outputs off with `jay idle set 1s`, which this
/// grace period of zero makes immediate.
pub fn setup() {
    set_idle(None);
    set_idle_grace_period(Duration::ZERO);
}
