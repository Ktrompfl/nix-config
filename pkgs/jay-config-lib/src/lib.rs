use jay_config::{
    config,
    exec::{Command, unset_env},
    input::{FallbackOutputMode, FocusFollowsMouseMode, get_default_seat},
    keyboard::syms::SYM_Super_L,
    set_middle_click_paste_enabled, set_show_titles, set_split_reuses_container,
    video::on_graphics_initialized,
    workspace::{WorkspaceDisplayOrder, set_workspace_display_order},
};

mod bar;
mod clients;
mod idle;
mod inputs;
mod outputs;
mod shortcuts;
mod theme;
mod windows;

pub fn exec(prog: &str, args: &[&str]) {
    let mut command = Command::new(prog);
    for arg in args {
        command.arg(arg);
    }
    command.spawn();
}

fn configure() {
    let seat = get_default_seat();
    seat.set_window_management_key(SYM_Super_L);
    seat.set_focus_follows_mouse_mode(FocusFollowsMouseMode::True);
    #[allow(deprecated)]
    seat.unstable_set_mouse_follows_focus(true);
    seat.set_fallback_output_mode(FallbackOutputMode::Focus);
    set_workspace_display_order(WorkspaceDisplayOrder::Sorted);
    set_middle_click_paste_enabled(false);
    set_show_titles(true);
    set_split_reuses_container(true);

    bar::setup();
    clients::setup();
    idle::setup();
    inputs::setup();
    outputs::setup();
    shortcuts::setup();
    theme::setup();
    windows::setup();

    // jay.service, and with it graphical-session.target, is only ready once jay
    // can render; clients committing shm buffers before then are disconnected.
    // Nothing else jay spawns gets to report for the service.
    let notify_socket = std::env::var("NOTIFY_SOCKET").ok();
    unset_env("NOTIFY_SOCKET");
    on_graphics_initialized(move || {
        if let Some(socket) = notify_socket {
            Command::new("systemd-notify")
                .arg("--ready")
                .env("NOTIFY_SOCKET", &socket)
                .spawn();
        }
    });
}

config!(configure);
