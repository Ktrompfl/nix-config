//! What happens to a window when it is mapped.

use jay_config::{
    get_workspace,
    window::{MatchedWindow, TileState, WindowCriterion},
};

use crate::outputs;

const NINJABRAIN_TITLE: &str = "Xwayland on :77";

pub fn setup() {
    let wl_mirror = WindowCriterion::All(&[
        WindowCriterion::AppId("at.yrlf.wl_mirror"),
        WindowCriterion::JustMapped,
    ])
    .to_matcher();

    wl_mirror.set_auto_focus(false);
    wl_mirror.bind(|matched: MatchedWindow| {
        let window = matched.window();
        let workspace = get_workspace("0");
        window.set_workspace(workspace);

        let beamer = outputs::connector("beamer");
        if beamer.exists() {
            workspace.move_to_output(beamer);
        }

        window.set_fullscreen(true);
    });

    let ninjabrain = WindowCriterion::All(&[
        WindowCriterion::AppId("org.freedesktop.Xwayland"),
        WindowCriterion::Title(NINJABRAIN_TITLE),
        WindowCriterion::JustMapped,
    ])
    .to_matcher();

    ninjabrain.set_auto_focus(false);
    ninjabrain.set_initial_tile_state(TileState::Floating);
}
