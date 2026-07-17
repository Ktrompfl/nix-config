use std::time::Duration;

use jay_config::timer::duration_until_wall_clock_is_multiple_of;

use super::schedule::repeat_with_initial;

fn now() -> String {
    chrono::Local::now().format("%a %d %b %H:%M").to_string()
}

pub fn run(on_update: impl Fn(String) + 'static) {
    // Align the first tick to the next minute boundary rather than firing
    // every minute counted from whenever the compositor happened to start.
    let period = Duration::from_secs(60);
    let initial = duration_until_wall_clock_is_multiple_of(period);
    repeat_with_initial("bar-clock", initial, period, move || on_update(now()));
}
