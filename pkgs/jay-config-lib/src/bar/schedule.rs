use std::time::Duration;

use jay_config::timer::get_timer;

/// Runs `f` once immediately, then every `period` starting after `initial`
/// (which must not be zero).
///
/// A zero-duration `initial` would mean the timer never ticks again after
/// the immediate call above: jay's timers are backed by Linux `timerfd`s,
/// and `timerfd_settime` treats an initial expiration of zero as a request
/// to disarm the timer rather than "fire right away".
pub fn repeat_with_initial(name: &str, initial: Duration, period: Duration, mut f: impl FnMut() + 'static) {
    let initial = if initial.is_zero() { period } else { initial };
    let timer = get_timer(name);
    f();
    timer.on_tick(move || f());
    timer.repeated(initial, period);
}
