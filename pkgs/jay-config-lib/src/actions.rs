//! The actions that more than one shortcut - or the compositor itself -
//! refers to. The toml side declares them by name in `actions.nix`.

use std::cell::Cell;

use jay_config::set_idle;

use crate::{IDLE_TIMEOUT, bar, exec};

pub fn lock() {
    exec("swaylock", &["--daemonize"]);
}

pub fn suspend() {
    exec("systemctl", &["suspend-then-hibernate"]);
}

thread_local! {
    static IDLE_INHIBITED: Cell<bool> = const { Cell::new(false) };
}

/// Disables the idle timeout until it is called again. The toml side has no
/// state of its own and has to build this out of two actions that redefine
/// which one the shortcut runs next.
pub fn toggle_idle_inhibitor() {
    let inhibited = IDLE_INHIBITED.with(|state| {
        state.set(!state.get());
        state.get()
    });
    log::info!("idle inhibitor {}", if inhibited { "on" } else { "off" });

    set_idle(if inhibited { None } else { Some(IDLE_TIMEOUT) });
    bar::set_idle_inhibitor(inhibited);
}
