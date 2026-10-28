//! Mirrors the laptop panel onto external outputs with wl-present, fullscreen
//! on workspace 0.

use std::cell::Cell;

use jay_config::{
    get_workspace,
    video::{Connector, connectors},
    window::{MatchedWindow, Window, WindowCriterion},
};

use crate::{exec, outputs::LAPTOP_PANEL};

thread_local! {
    /// clients spawned before graphics are initialized may be disconnected
    static READY: Cell<bool> = const { Cell::new(false) };
    /// wl-present was spawned, but its window is not mapped yet
    static PENDING: Cell<bool> = const { Cell::new(false) };
    static WINDOW: Cell<Option<Window>> = const { Cell::new(None) };
}

/// An output other than the laptop panel, if the panel is connected.
/// `disconnected` is still reported as connected while its event is handled.
fn external(disconnected: Option<Connector>) -> Option<Connector> {
    let panel = LAPTOP_PANEL.find()?;
    connectors()
        .into_iter()
        .find(|&c| c.connected() && c != panel && Some(c) != disconnected)
}

/// Starts mirroring while an external output is connected, stops otherwise.
pub fn update(disconnected: Option<Connector>) {
    if !READY.get() {
        return;
    }
    let window = WINDOW.get().filter(|w| w.exists());
    match (external(disconnected), window) {
        (Some(_), None) if !PENDING.get() => {
            PENDING.set(true);
            exec("wl-present", &["mirror", &LAPTOP_PANEL.connector().name()]);
        }
        (None, Some(window)) => window.close(),
        (None, None) => PENDING.set(false),
        _ => {}
    }
}

/// Call once graphics are initialized; mirrors outputs connected before then.
pub fn start() {
    READY.set(true);
    update(None);
}

pub fn setup() {
    let wl_mirror = WindowCriterion::All(&[
        WindowCriterion::AppId("at.yrlf.wl_mirror"),
        WindowCriterion::JustMapped,
    ])
    .to_matcher();
    wl_mirror.set_auto_focus(false);
    wl_mirror.bind(|matched: MatchedWindow| {
        let window = matched.window();
        WINDOW.set(Some(window));
        PENDING.set(false);
        let workspace = get_workspace("0");
        window.set_workspace(workspace);
        if let Some(output) = external(None) {
            workspace.move_to_output(output);
        }
        window.set_fullscreen(true);
    });
}
