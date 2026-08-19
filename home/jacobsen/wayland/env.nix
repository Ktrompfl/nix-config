config: {
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

  ELECTRON_OZONE_PLATFORM_HINT = "wayland";
  OZONE_PLATFORM = "wayland";
  NIXOS_OZONE_WL = "1";

  XCURSOR_THEME = config.theme.cursor.name;
  XCURSOR_SIZE = toString config.theme.cursor.size;

  GTK_THEME = "adw-gtk3";
  QT_QPA_PLATFORMTHEME = "qtct";
  QT_STYLE_OVERRIDE = "kvantum";
}
