use std::{cell::Cell, time::Duration};

use jay_config::{on_idle, set_idle, set_idle_grace_period};

use crate::{bar, exec};

const TIMEOUT: Duration = Duration::from_secs(10 * 60);

thread_local! {
    static INHIBITED: Cell<bool> = const { Cell::new(false) };
}

pub fn suspend() {
    exec("systemctl", &["suspend-then-hibernate"]);
}

pub fn inhibited() -> bool {
    INHIBITED.get()
}

pub fn toggle_inhibitor() {
    let inhibited = !INHIBITED.get();
    INHIBITED.set(inhibited);
    set_idle(if inhibited { None } else { Some(TIMEOUT) });
    bar::set_idle_inhibitor(inhibited);
}

pub fn setup() {
    set_idle(Some(TIMEOUT));
    set_idle_grace_period(Duration::from_secs(15));
    on_idle(suspend);
}
