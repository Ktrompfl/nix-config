//! The environment that spawned programs inherit. The theme-derived values
//! come from theme.toml, see [`crate::theme`].

use jay_config::exec::set_env;

use crate::theme;

pub fn setup() {
    let env = [
        // wayland backends
        ("CLUTTER_BACKEND", "wayland"),
        ("GDK_BACKEND", "wayland,x11,*"),
        ("QT_QPA_PLATFORM", "wayland;xcb"),
        ("SDL_VIDEODRIVER", "wayland"),
        ("_JAVA_AWT_WM_NONREPARENTING", "1"),
        ("MOZ_ENABLE_WAYLAND", "1"),
        ("QT_WAYLAND_DISABLE_WINDOWDECORATION", "1"),
        ("QT_AUTO_SCREEN_SCALE_FACTOR", "1"),
        ("WLR_NO_HARDWARE_CURSORS", "1"),
        // electron apps
        ("ELECTRON_OZONE_PLATFORM_HINT", "wayland"),
        ("OZONE_PLATFORM", "wayland"),
        ("NIXOS_OZONE_WL", "1"),
        // cursor
        ("XCURSOR_THEME", theme::string("cursor_theme")),
        ("XCURSOR_SIZE", theme::string("cursor_size")),
        // gtk theme
        ("GTK_THEME", theme::string("gtk_theme")),
        // qt theme
        ("QT_QPA_PLATFORMTHEME", theme::string("qt_platform_theme")),
        ("QT_STYLE_OVERRIDE", theme::string("qt_style")),
    ];

    for (key, value) in env {
        set_env(key, value);
    }
}
