use jay_config::{
    Workspace, get_workspace,
    video::Connector,
    window::{MatchedWindow, Window, WindowCriterion},
};

use crate::outputs;

fn move_to_output(window: Window, ws_name: &str, output: Connector) {
    let ws: Workspace = get_workspace(ws_name);
    window.set_workspace(ws);
    if output.exists() {
        ws.move_to_output(output);
    }
}

pub fn setup() {
    WindowCriterion::AppId("spotify").bind(|matched: MatchedWindow| {
        move_to_output(matched.window(), "5", outputs::vertical());
    });

    WindowCriterion::AppId("vesktop").bind(|matched: MatchedWindow| {
        move_to_output(matched.window(), "5", outputs::vertical());
    });

    WindowCriterion::AppId("firefox").bind(|matched: MatchedWindow| {
        move_to_output(matched.window(), "1", outputs::horizontal());
    });

    let wl_mirror = WindowCriterion::All(&[
        WindowCriterion::AppId("at.yrlf.wl_mirror"),
        WindowCriterion::JustMapped,
    ])
    .to_matcher();
    wl_mirror.set_auto_focus(false);
    wl_mirror.bind(|matched: MatchedWindow| {
        let window = matched.window();
        move_to_output(window, "0", outputs::beamer());
        window.set_fullscreen(true);
    });
}
