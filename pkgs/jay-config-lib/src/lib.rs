//! The shared library configuration. It is split into the same parts as the
//! toml configuration in home/jacobsen/wayland/jay, so that the two can be
//! read side by side: this file corresponds to its `default.nix`, `bar.rs` to
//! its `bar.nix`, and so on. Wherever one side cannot do what the other does,
//! the comment on both sides says so.

use std::time::Duration;

use jay_config::{config, exec::Command};

mod actions;
mod bar;
mod behavior;
mod clients;
mod inputs;
mod outputs;
mod shortcuts;
mod theme;
mod windows;

/// Shared by [`behavior`], which arms the timeout, and [`actions`], which
/// restores it when the idle inhibitor is switched off again.
pub const IDLE_TIMEOUT: Duration = Duration::from_secs(10 * 60);

/// screen goes black during grace period before idle action and output disable
pub const IDLE_GRACE_PERIOD: Duration = Duration::from_secs(15);

/// Runs a program. Unlike the toml configuration, which can name the store
/// path of every program it runs, this has to look them up in `PATH`;
/// home/jacobsen/wayland/jay installs the ones that are not part of the
/// session anyway.
pub fn exec(prog: &str, args: &[&str]) {
    let mut command = Command::new(prog);
    for arg in args {
        command.arg(arg);
    }
    command.spawn();
}

fn configure() {
    bar::setup();
    behavior::setup();
    clients::setup();
    inputs::setup();
    outputs::setup();
    shortcuts::setup();
    theme::setup();
    windows::setup();
}

config!(configure);
