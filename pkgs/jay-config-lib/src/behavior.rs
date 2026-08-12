//! Compositor-wide behaviour that belongs to no particular device, window, or
//! key.

use jay_config::{
    input::{FallbackOutputMode, FocusFollowsMouseMode, get_default_seat},
    keyboard::syms::SYM_Super_L,
    on_idle, set_idle, set_idle_grace_period, set_middle_click_paste_enabled, set_show_titles,
    workspace::{WorkspaceDisplayOrder, set_workspace_display_order},
};

use crate::{IDLE_GRACE_PERIOD, IDLE_TIMEOUT, actions};

pub fn setup() {
    let seat = get_default_seat();

    // logo uses different symbol names
    seat.set_window_management_key(SYM_Super_L);

    seat.set_focus_follows_mouse_mode(FocusFollowsMouseMode::True);
    #[allow(deprecated)]
    seat.unstable_set_mouse_follows_focus(true);
    // more useful with mouse-follows-focus
    seat.set_fallback_output_mode(FallbackOutputMode::Focus);
    set_workspace_display_order(WorkspaceDisplayOrder::Sorted);
    set_middle_click_paste_enabled(false);
    set_show_titles(true);

    // The toml side additionally sets `split-reuses-container`, which this
    // version of the jay-config crate does not know about.

    set_idle(Some(IDLE_TIMEOUT));
    set_idle_grace_period(IDLE_GRACE_PERIOD);
    on_idle(|| {
        log::info!("idle timeout reached: suspending");
        actions::suspend();
    });
}
