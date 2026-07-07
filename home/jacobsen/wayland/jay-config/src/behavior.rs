use std::time::Duration;

use jay_config::{
    exec::Command,
    input::{FallbackOutputMode, FocusFollowsMouseMode, get_default_seat},
    keyboard::syms::SYM_Super_L,
    on_idle, set_idle, set_idle_grace_period, set_middle_click_paste_enabled, set_show_titles,
    workspace::{WorkspaceDisplayOrder, set_workspace_display_order},
};

pub fn setup() {
    let seat = get_default_seat();

    // logo uses a different symbol name than the other modifier keys
    seat.set_window_management_key(SYM_Super_L);

    seat.set_focus_follows_mouse_mode(FocusFollowsMouseMode::True);
    #[allow(deprecated)]
    seat.unstable_set_mouse_follows_focus(true);
    // more useful with mouse-follows-focus
    seat.set_fallback_output_mode(FallbackOutputMode::Focus);
    set_workspace_display_order(WorkspaceDisplayOrder::Sorted);

    set_idle(Some(Duration::from_secs(10 * 60)));
    // screen goes black during grace period before idle action and output disable
    set_idle_grace_period(Duration::from_secs(15));
    on_idle(|| {
        log::info!("idle timeout reached: suspending");
        Command::new("systemctl").arg("suspend").spawn();
    });

    set_show_titles(true);
    set_middle_click_paste_enabled(false);
}
