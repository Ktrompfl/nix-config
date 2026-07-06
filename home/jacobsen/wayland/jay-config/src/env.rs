use jay_config::exec::set_env;

use crate::theme;

pub fn setup() {
    // wayland backends
    set_env("CLUTTER_BACKEND", "wayland");
    set_env("GDK_BACKEND", "wayland,x11,*");
    set_env("QT_QPA_PLATFORM", "wayland;xcb");
    set_env("SDL_VIDEODRIVER", "wayland");

    set_env("_JAVA_AWT_WM_NONREPARENTING", "1");
    set_env("MOZ_ENABLE_WAYLAND", "1");
    set_env("QT_WAYLAND_DISABLE_WINDOWDECORATION", "1");
    set_env("QT_AUTO_SCREEN_SCALE_FACTOR", "1");
    set_env("WLR_NO_HARDWARE_CURSORS", "1");

    // electron apps
    set_env("ELECTRON_OZONE_PLATFORM_HINT", "wayland");
    set_env("OZONE_PLATFORM", "wayland");
    set_env("NIXOS_OZONE_WL", "1");

    // cursor
    set_env("XCURSOR_THEME", theme::cursor_theme());
    set_env("XCURSOR_SIZE", theme::cursor_size());

    // gtk theme
    set_env("GTK_THEME", theme::gtk_theme());

    // qt theme
    set_env("QT_QPA_PLATFORMTHEME", theme::qt_platform_theme());
    set_env("QT_STYLE_OVERRIDE", theme::qt_style());
}
