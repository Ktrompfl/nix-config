{
  config,
  inputs,
  lib,
  pkgs,
  ...
}:
let
  # --- action constructors ---

  exec = command: {
    type = "exec";
    exec = command;
  };

  # `exec.shell` hands the command to `$SHELL`, which is not necessarily a
  # POSIX shell, so the scripts below name their interpreter themselves.
  sh =
    script:
    exec {
      prog = "sh";
      args = [
        "-c"
        script
      ];
    };

  # --- bar ---

  # The two bar segments that reflect compositor state - the input mode and
  # the idle inhibitor - cannot be observed from the outside, so they are
  # pushed into i3status-rust's `custom_dbus` blocks whenever the state
  # changes. See ../../programs/i3status-rust.nix for the receiving end.
  setBlockText =
    path: text:
    exec [
      (lib.getExe' pkgs.systemd "busctl")
      "--user"
      "call"
      "rs.i3status"
      path
      "rs.i3status.custom"
      "SetText"
      "ss"
      text
      ""
    ];

  idleInhibitorOnIcon = "";
  idleInhibitorOffIcon = "";

  # A `custom_dbus` block stays invisible until something has pushed a value
  # into it, so both of them are seeded once the bar is up. i3status-rust
  # only claims the bus name a moment after jay spawns it, hence the retry.
  barInit = pkgs.writeShellApplication {
    name = "jay-bar-init";
    runtimeInputs = with pkgs; [
      coreutils
      systemd
    ];
    text = ''
      set_text() {
        busctl --user call rs.i3status "$1" rs.i3status.custom SetText ss "$2" "" >/dev/null 2>&1
      }

      for _ in {1..100}; do
        if set_text /mode NORMAL && set_text /idle_inhibitor '${idleInhibitorOffIcon}'; then
          exit 0
        fi
        sleep 0.1
      done

      exit 1
    '';
  };

  # --- idle ---

  idleTimeout = {
    minutes = 10;
    # screen goes black during grace period before idle action and output disable
    grace-period.seconds = 15;
  };
in
{
  imports = [
    inputs.jay.homeManagerModules.default

    ./clients.nix
    ./inputs.nix
    ./outputs.nix
    ./shortcuts.nix
    ./theme.nix
    ./windows.nix
  ];

  # Helpers the modules above share. Anything only one of them needs stays in
  # that module.
  _module.args.jayLib = {
    inherit
      barInit
      exec
      setBlockText
      sh
      ;
  };

  wayland.windowManager.jay = {
    enable = true;
    # library = pkgs.jay-config-lib;

    settings = {
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

      actions = {
        lock = exec [
          (lib.getExe pkgs.swaylock)
          "--daemonize"
        ];

        # falls back to plain suspend when there is nothing to hibernate into
        suspend = sh ''
          if grep -qw disk /sys/power/state; then
            systemctl suspend-then-hibernate
          else
            systemctl suspend
          fi
        '';

        # The toml config has no state of its own, so the inhibitor toggle is
        # built out of two actions that redefine which one the shortcut runs
        # next. Anything else that changes the idle timeout (e.g. `jay idle`)
        # desynchronizes this.
        inhibit-idle = [
          {
            type = "configure-idle";
            idle.minutes = 0; # disables the timeout entirely
          }
          (setBlockText "/idle_inhibitor" idleInhibitorOnIcon)
          {
            type = "define-action";
            name = "toggle-idle-inhibitor";
            action = "$uninhibit-idle";
          }
        ];
        uninhibit-idle = [
          {
            type = "configure-idle";
            idle = idleTimeout;
          }
          (setBlockText "/idle_inhibitor" idleInhibitorOffIcon)
          {
            type = "define-action";
            name = "toggle-idle-inhibitor";
            action = "$inhibit-idle";
          }
        ];
        toggle-idle-inhibitor = "$inhibit-idle";
      };

      # --- behavior ---

      focus-follows-mouse = true;
      unstable-mouse-follows-focus = true;
      fallback-output-mode = "focus"; # more useful with mouse-follows-focus
      workspace-display-order = "sorted";
      middle-click-paste = false;
      split-reuses-container = true;

      idle = idleTimeout;
      on-idle = "$suspend";

      # --- bar ---

      status = {
        format = "i3bar";
        exec = [
          (lib.getExe' config.programs.i3status-rust.package "i3status-rs")
          "config-jay.toml"
        ];
        # every block brings its own padding, see ../../programs/i3status-rust.nix
        i3bar-separator = "";
      };
    };
  };

  # extra packages used in the jay config
  home.packages = with pkgs; [
    playerctl
    wl-mirror
  ];

  # persist logs and session management
  preservation.preserveAt.state-dir.directories = [ ".local/share/jay" ];
}
