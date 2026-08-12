//! What happens to a window when it is mapped.

use jay_config::{
    get_workspace,
    window::{MatchedWindow, WindowCriterion},
};

use crate::outputs;

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
}
