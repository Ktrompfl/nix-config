{ config, pkgs, ... }:
{
  home.packages = [
    pkgs.app2unit # uwsm app launcher
  ];

  # uwsm environment
  xdg.configFile."uwsm/env".text = /* sh */ ''
    export CLUTTER_BACKEND=wayland
    export GDK_BACKEND=wayland,x11,*
    export QT_QPA_PLATFORM=wayland;xcb
    export SDL_VIDEODRIVER=wayland

    export _JAVA_AWT_WM_NONREPARENTING=1
    export MOZ_ENABLE_WAYLAND=1
    export QT_WAYLAND_DISABLE_WINDOWDECORATION=1
    export QT_AUTO_SCREEN_SCALE_FACTOR=1
    export WLR_NO_HARDWARE_CURSORS=1

    export ELECTRON_OZONE_PLATFORM_HINT=wayland
    export OZONE_PLATFORM=wayland
    export NIXOS_OZONE=1

    export GTK_THEME=${config.gtk.theme.name}
    export XCURSOR_THEME=${config.home.pointerCursor.name}
    export XCURSOR_SIZE=${toString config.home.pointerCursor.size}

    export QT_QPA_PLATFORMTHEME=${config.qt.platformTheme.name}
    export QT_STYLE_OVERRIDE=${config.qt.style.name}

    export TERMINAL=footclient

    export APP2UNIT_SLICES="a=app-graphical.slice b=background-graphical.slice s=session-graphical.slice"
  '';
}
