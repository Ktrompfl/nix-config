use std::time::Duration;

use jay_config::timer::get_timer;

/// Runs `f` once immediately, then every `period`.
///
/// The first scheduled tick also happens after `period`, not sooner: jay's
/// timers are backed by Linux `timerfd`s, and `timerfd_settime` treats an
/// initial expiration of zero as a request to disarm the timer rather than
/// "fire right away", so a zero-duration initial delay would mean the timer
/// never ticks again after the immediate call above.
pub fn repeat(name: &str, period: Duration, f: impl FnMut() + 'static) {
    repeat_with_initial(name, period, period, f)
}

/// Like [`repeat`], but with an explicit delay before the first scheduled
/// tick (which must not be zero; see [`repeat`]'s doc comment).
pub fn repeat_with_initial(name: &str, initial: Duration, period: Duration, mut f: impl FnMut() + 'static) {
    let initial = if initial.is_zero() { period } else { initial };
    let timer = get_timer(name);
    f();
    timer.on_tick(move || f());
    timer.repeated(initial, period);
}
