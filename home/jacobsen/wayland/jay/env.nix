{ config, ... }:
{
  # The environment that spawned programs inherit. The shared library
  # configuration reads the theme-derived values back out of theme.toml, see
  # default.nix.
  env = {
    # wayland backends
    CLUTTER_BACKEND = "wayland";
    GDK_BACKEND = "wayland,x11,*";
    QT_QPA_PLATFORM = "wayland;xcb";
    SDL_VIDEODRIVER = "wayland";

    _JAVA_AWT_WM_NONREPARENTING = "1";
    MOZ_ENABLE_WAYLAND = "1";
    QT_WAYLAND_DISABLE_WINDOWDECORATION = "1";
    QT_AUTO_SCREEN_SCALE_FACTOR = "1";
    WLR_NO_HARDWARE_CURSORS = "1";

    # electron apps
    ELECTRON_OZONE_PLATFORM_HINT = "wayland";
    OZONE_PLATFORM = "wayland";
    NIXOS_OZONE_WL = "1";

    # cursor
    XCURSOR_THEME = config.home.pointerCursor.name;
    XCURSOR_SIZE = toString config.home.pointerCursor.size;

    # gtk theme
    GTK_THEME = config.gtk.theme.name;

    # qt theme
    QT_QPA_PLATFORMTHEME = config.qt.platformTheme.name;
    QT_STYLE_OVERRIDE = config.qt.style.name;
  };
}
