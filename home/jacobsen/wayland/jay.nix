{
  config,
  lib,
  pkgs,
  ...
}:
let
  # screenshot script to work around the following problems
  # - jay has no proper screenshot selections yet, see https://github.com/mahkoh/jay/issues/564
  # - grim fails for multi-monitor setups with transforms, see https://todo.sr.ht/~emersion/grim/100
  jay-screenshot = pkgs.writeShellScript "jay-screenshot" /* sh */ ''
    # --- require type argument ---
    if [ $# -lt 1 ]; then
      echo "Usage: $0 <window|region|workspace>" >&2
      exit 1
    fi

    type="$1"

    # --- determine region ---
    case "$type" in
      workspace)
        region=$(jay tree query select-workspace | awk '
          /pos:/ {
            gsub(" ", "");        # remove spaces
            sub("pos:", "");      # remove "pos:" prefix
            split($0, a, "\\+");  # split on "+"
            split(a[1], xy, "x"); # first part = XxY
            split(a[2], wh, "x"); # second part = WxH
            print wh[1] "x" wh[2] "+" xy[1] "+" xy[2]
          }
        ')
        ;;
      window)
        region=$(jay tree query select-window | awk '
          /pos:/ {
            gsub(" ", "");        # remove spaces
            sub("pos:", "");      # remove "pos:" prefix
            split($0, a, "\\+");  # split on "+"
            split(a[1], xy, "x"); # first part = XxY
            split(a[2], wh, "x"); # second part = WxH
            print wh[1] "x" wh[2] "+" xy[1] "+" xy[2]
          }
        ')
        ;;
      region)
        # interactively select region with slurp
        region=$(${lib.getExe pkgs.slurp} | awk '{split($1,a,","); split($2,b,"x"); printf "%sx%s+%s+%s\n", b[1], b[2], a[1], a[2]}');
        ;;
      *)
        echo "Error: unknown type '$type'. Must be one of: window, region, workspace" >&2
        exit 1
        ;;
    esac

    # --- validate region ---
    if ! [[ $region =~ ^-?[0-9]+x-?[0-9]+\+-?[0-9]+\+-?[0-9]+$ ]]; then
      echo "Error: invalid region: '$region'" >&2
      exit 1
    fi

    # --- setup output file ---
    if [ -n "$XDG_SCREENSHOTS_DIR" ]; then
      SCREENSHOTS_DIR="$XDG_SCREENSHOTS_DIR"
    else
      SCREENSHOTS_DIR="$HOME/Pictures/screenshots"
    fi
    output="$SCREENSHOTS_DIR/$(date +'%Y-%m-%d-%H%M%S.png')";

    # --- take global screenshot ---
    TMPFILE=$(mktemp --suffix=.png)
    jay screenshot "$TMPFILE"

    # --- crop screenshot to region ---
    ${pkgs.imagemagick}/bin/magick "$TMPFILE" -crop "$region" +repage "$output"

    # --- copy to clipboard ---
    wl-copy --type "image/png" < "$output"

    # --- notify ---
    ${pkgs.libnotify}/bin/notify-send --app-name="Screenshot" --urgency=normal --expire-time=3000 --icon="$output"  "Screenshot Saved" "Screenshot saved to $output and copied to clipboard"
  '';
in
{
  wayland.windowManager.jay = {
    enable = true;
    settings =
      let
        modifier = "logo";
      in
      {
        env = {
          # enable wayland backends
          CLUTTER_BACKEND = "wayland";
          GDK_BACKEND = "wayland,x11,*";
          QT_QPA_PLATFORM = "wayland;xcb";
          SDL_VIDEODRIVER = "wayland";

          _JAVA_AWT_WM_NONREPARENTING = "1";
          MOZ_ENABLE_WAYLAND = "1";
          QT_WAYLAND_DISABLE_WINDOWDECORATION = "1";
          QT_AUTO_SCREEN_SCALE_FACTOR = "1";
          WLR_NO_HARDWARE_CURSORS = "1";

          # enable wayland for electron apps
          ELECTRON_OZONE_PLATFORM_HINT = "wayland";
          OZONE_PLATFORM = "wayland";
          NIXOS_OZONE = "1";

          # cursor theme
          XCURSOR_THEME = "${config.home.pointerCursor.name}";
          XCURSOR_SIZE = "${toString config.home.pointerCursor.size}";

          # gtk theme
          GTK_THEME = "${config.gtk.theme.name}";

          # qt theme
          QT_QPA_PLATFORMTHEME = "${config.qt.platformTheme.name}";
          QT_STYLE_OVERRIDE = "${config.qt.style.name}";
        };

        actions = {
          lock = {
            type = "exec";
            exec = {
              prog = "${lib.getExe pkgs.swaylock}";
              privileged = true;
            };
          };
        };

        shortcuts = {
          ## tty controls
          # The switch-to-vt action switches to a different virtual terminal.
          "ctrl-alt-F1" = {
            type = "switch-to-vt";
            num = 1;
          };
          "ctrl-alt-F2" = {
            type = "switch-to-vt";
            num = 2;
          };
          "ctrl-alt-F3" = {
            type = "switch-to-vt";
            num = 3;
          };
          "ctrl-alt-F4" = {
            type = "switch-to-vt";
            num = 4;
          };
          "ctrl-alt-F5" = {
            type = "switch-to-vt";
            num = 5;
          };
          "ctrl-alt-F6" = {
            type = "switch-to-vt";
            num = 6;
          };
          "ctrl-alt-F7" = {
            type = "switch-to-vt";
            num = 7;
          };
          "ctrl-alt-F8" = {
            type = "switch-to-vt";
            num = 8;
          };
          "ctrl-alt-F9" = {
            type = "switch-to-vt";
            num = 9;
          };
          "ctrl-alt-F10" = {
            type = "switch-to-vt";
            num = 10;
          };
          "ctrl-alt-F11" = {
            type = "switch-to-vt";
            num = 11;
          };
          "ctrl-alt-F12" = {
            type = "switch-to-vt";
            num = 12;
          };

          ## compositor controls
          # The quit action terminates the compositor.
          "${modifier}-shift-q" = "quit";
          # The reload-config-toml action reloads the TOML configuration file.
          "${modifier}-shift-r" = "reload-config-toml";

          ## window controls
          # The close action requests the currently focused window to close.
          "${modifier}-q" = "close";

          # The focus-X actions move the keyboard focus to next window on the X.
          "${modifier}-h" = "focus-left";
          "${modifier}-j" = "focus-down";
          "${modifier}-k" = "focus-up";
          "${modifier}-l" = "focus-right";
          "${modifier}-Left" = "focus-left";
          "${modifier}-Down" = "focus-down";
          "${modifier}-Up" = "focus-up";
          "${modifier}-Right" = "focus-right";

          # The move-X actions move window that has the keyboard focus to the X.
          "${modifier}-shift-h" = "move-left";
          "${modifier}-shift-j" = "move-down";
          "${modifier}-shift-k" = "move-up";
          "${modifier}-shift-l" = "move-right";
          "${modifier}-shift-Left" = "move-left";
          "${modifier}-shift-Down" = "move-down";
          "${modifier}-shift-Up" = "move-up";
          "${modifier}-shift-Right" = "move-right";

          # The move-to-output action moves the window that has the keyboard focus to the next output in the specified direction.
          "${modifier}-shift-ctrl-h" = {
            type = "move-to-output";
            direction = "left";
          };
          "${modifier}-shift-ctrl-j" = {
            type = "move-to-output";
            direction = "down";
          };
          "${modifier}-shift-ctrl-k" = {
            type = "move-to-output";
            direction = "up";
          };
          "${modifier}-shift-ctrl-l" = {
            type = "move-to-output";
            direction = "right";
          };
          "${modifier}-shift-ctrl-Left" = {
            type = "move-to-output";
            direction = "left";
          };
          "${modifier}-shift-ctrl-Down" = {
            type = "move-to-output";
            direction = "down";
          };
          "${modifier}-shift-ctrl-Up" = {
            type = "move-to-output";
            direction = "up";
          };
          "${modifier}-shift-ctrl-Right" = {
            type = "move-to-output";
            direction = "right";
          };

          # The toggle-fullscreen action toggles the current window between
          # windowed and fullscreen.
          "${modifier}-f" = "toggle-fullscreen";
          # The toggle-floating action changes the currently focused window between
          # floating and tiled.
          "${modifier}-space" = "toggle-floating";

          # The toggle-mono action changes whether the current container shows
          # a single window or all windows next to each other.
          "${modifier}-m" = "toggle-mono";
          # The toggle-split action changes the split direction of the current
          # container.
          "${modifier}-v" = "toggle-split";
          # The split-X action places the currently focused window in a container
          # and sets the split direction of the container to X.
          # "${modifier}-b" = "split-horizontal";
          # "${modifier}-n" = "split-vertical";

          # Disable the currently active pointer constraint, allowing you to move the pointer outside the window.
          # The constraint will be re-enabled when the pointer re-enters the window.
          "${modifier}-Escape" = "disable-pointer-constraint";

          ## workspace controls
          # The show-workspace action switches to a workspace. If the workspace is not
          # currently being used, it is created on the output that contains the pointer.
          "${modifier}-0" = {
            type = "show-workspace";
            name = "0";
          };
          "${modifier}-1" = {
            type = "show-workspace";
            name = "1";
          };
          "${modifier}-2" = {
            type = "show-workspace";
            name = "2";
          };
          "${modifier}-3" = {
            type = "show-workspace";
            name = "3";
          };
          "${modifier}-4" = {
            type = "show-workspace";
            name = "4";
          };
          "${modifier}-5" = {
            type = "show-workspace";
            name = "5";
          };
          "${modifier}-6" = {
            type = "show-workspace";
            name = "6";
          };
          "${modifier}-7" = {
            type = "show-workspace";
            name = "7";
          };
          "${modifier}-8" = {
            type = "show-workspace";
            name = "8";
          };
          "${modifier}-9" = {
            type = "show-workspace";
            name = "9";
          };

          # The move-to-workspace action moves the currently focused window to a workspace.
          "${modifier}-shift-0" = {
            type = "move-to-workspace";
            name = "0";
          };
          "${modifier}-shift-1" = {
            type = "move-to-workspace";
            name = "1";
          };
          "${modifier}-shift-2" = {
            type = "move-to-workspace";
            name = "2";
          };
          "${modifier}-shift-3" = {
            type = "move-to-workspace";
            name = "3";
          };
          "${modifier}-shift-4" = {
            type = "move-to-workspace";
            name = "4";
          };
          "${modifier}-shift-5" = {
            type = "move-to-workspace";
            name = "5";
          };
          "${modifier}-shift-6" = {
            type = "move-to-workspace";
            name = "6";
          };
          "${modifier}-shift-7" = {
            type = "move-to-workspace";
            name = "7";
          };
          "${modifier}-shift-8" = {
            type = "move-to-workspace";
            name = "8";
          };
          "${modifier}-shift-9" = {
            type = "move-to-workspace";
            name = "9";
          };

          ## modes
          "${modifier}-p" = {
            # Temporarily pushes an input mode on top of the input-mode stack. The new mode will automatically be popped when the next shortcut is invoked.
            type = "latch-mode";
            name = "power";
          };
          "${modifier}-r" = {
            # Pushes an input mode on top of the input-mode stack. The mode can be popped with the pop-mode action.
            type = "push-mode";
            name = "resize";
          };

          ## media keys
          # audio control (wireplumber)
          XF86AudioRaiseVolume = {
            type = "exec";
            exec = [
              "wpctl"
              "set-volume"
              "@DEFAULT_AUDIO_SINK@"
              "5%+"
              "--limit"
              "1.5"
            ];
          };
          XF86AudioLowerVolume = {
            type = "exec";
            exec = [
              "wpctl"
              "set-volume"
              "@DEFAULT_AUDIO_SINK@"
              "5%-"
              "--limit"
              "0.0"
            ];
          };
          XF86AudioMute = {
            type = "exec";
            exec = [
              "wpctl"
              "set-mute"
              "@DEFAULT_AUDIO_SINK@"
              "toggle"
            ];
          };
          XF86AudioMicMute = {
            type = "exec";
            exec = [
              "wpctl"
              "set-mute"
              "@DEFAULT_AUDIO_SOURCE@"
              "toggle"
            ];
          };

          # brightness control
          XF86MonBrightnessUp = {
            type = "exec";
            exec = [
              "${lib.getExe pkgs.brightnessctl}"
              "set"
              "4%+"
            ];
          };
          XF86MonBrightnessDown = {
            type = "exec";
            exec = [
              "${lib.getExe pkgs.brightnessctl}"
              "set"
              "4%-"
            ];
          };

          # player control
          XF86AudioPlay = {
            type = "exec";
            exec = [
              "${lib.getExe pkgs.playerctl}"
              "play-pause"
            ];
          };
          XF86AudioPause = {
            type = "exec";
            exec = [
              "${lib.getExe pkgs.playerctl}"
              "play-pause"
            ];
          };
          XF86AudioNext = {
            type = "exec";
            exec = [
              "${lib.getExe pkgs.playerctl}"
              "next"
            ];
          };
          XF86AudioPrev = {
            type = "exec";
            exec = [
              "${lib.getExe pkgs.playerctl}"
              "previous"
            ];
          };
          XF86AudioStop = {
            type = "exec";
            exec = [
              "${lib.getExe pkgs.playerctl}"
              "stop"
            ];
          };

          ## screenshot
          # interactively select workspace and create screenshot
          "${modifier}-s" = {
            type = "exec";
            exec = [
              "${jay-screenshot}"
              "workspace"
            ];
          };
          # interactively select window and create screenshot
          "${modifier}-shift-s" = {
            type = "exec";
            exec = [
              "${jay-screenshot}"
              "window"
            ];
          };
          # interactively select region and create screenshot
          "${modifier}-ctrl-s" = {
            type = "exec";
            exec = [
              "${jay-screenshot}"
              "region"
            ];
          };

          ## launch applications
          "${modifier}-Return" = {
            type = "exec";
            exec = [
              "${lib.getExe pkgs.app2unit}"
              "footclient"
            ];
          };
          "${modifier}-d" = {
            type = "exec";
            exec = "fuzzel";
          };
          "${modifier}-a" = {
            type = "exec";
            exec = [
              "swaync-client"
              "-t"
            ];
          };
        };

        # modes = {
        #   resize.shortcuts = {
        #     # return to default mode
        #     "${modifier}-r" = {
        #       type = "pop-mode";
        #     };
        #     "Escape" = {
        #       type = "pop-mode";
        #     };
        #   };

        #   power.shortcuts = {
        #     # "l" = mkCmd "${lib.getExe pkgs.swaylock}";
        #     # "s" = mkCmd "systemctl poweroff";
        #     # "r" = mkCmd "systemctl reboot";
        #     # "h" = mkCmd "systemctl suspend";
        #   };
        # };

        # see https://wiki.archlinux.org/title/X_keyboard_extension for xkb keymap settings
        keymaps = [
          {
            name = "laptop";
            map = ''
              xkb_keymap {
                xkb_keycodes { include "evdev+aliases(qwertz)" };
                xkb_types    { include "complete" };
                xkb_compat   { include "complete" };
                xkb_symbols  { include "pc+de(qwerty)+inet(evdev)+capslock(escape)" };
                xkb_geometry  { include "pc(pc105)"	};
              };
            '';
          }
          {
            name = "external";
            map = ''
               xkb_keymap {
                xkb_keycodes { include "evdev+aliases(qwerty)" };
                xkb_types    { include "complete" };
                xkb_compat   { include "complete" };
                xkb_symbols  { include "pc+us(altgr-intl)+inet(evdev)+capslock(escape)" };
                xkb_geometry  { include "pc(pc104)"	};
              };
            '';
          }
        ];

        # find names of inputs with 'jay input'
        inputs = [
          {
            match.is-pointer = true;
            tag = "mouse";
            accel-profile = "Flat";
            accel-speed = 0;
            left-handed = false;
            tap-enabled = true;
            natural-scrolling = false;
          }
          {
            match.name = "AT Translated Set 2 keyboard";
            tag = "laptop";
            keymap.name = "laptop";
          }
          {
            match.name = "I-CHIP YUNZII AL68 2.4G";
            tag = "Yunzii AL68";
            keymap.name = "external";
          }
          {
            match.name = "I-CHIP YUNZII AL68 2.4G Keyboard";
            keymap.name = "external";
          }
          {
            match.name = "I-CHIP YUNZII AL68 2.4G Mouse";
            keymap.name = "external";
          }
          {
            match.name = "Lid Switch";
            on-lid-closed = [
              {
                type = "named";
                name = "lock";
              }
              {
                type = "configure-connector";
                connector = {
                  match.name = "laptop-integrated";
                  enabled = false;
                };
              }
            ];
            on-lid-opened = {
              type = "configure-connector";
              connector = {
                match.name = "laptop-integrated";
                enabled = true;
              };
            };
          }
        ];

        # find names of outputs with 'jay randr'
        outputs = [
          {
            match.connector = "eDP-1";
            name = "laptop-integrated";
            mode = {
              width = 1920;
              height = 1080;
              refresh-rate = 60.049;
            };
          }
          {
            # TODO: setup correct color space / brightness / ...
            match.model = "VG270U P";
            name = "horizontal";
            x = 0;
            y = 240;
            scale = 1.0;
            mode = {
              width = 2560;
              height = 1440;
              refresh-rate = 143.995;
            };
            transform = "none";
            vrr.mode = "variant3"; # if this does not work try variant2
          }
          {
            match.model = "BenQ GL2480";
            name = "vertical";
            x = 2560;
            y = 0;
            scale = 1.0;
            mode = {
              width = 1920;
              height = 1080;
              refresh-rate = 60;
            };
            transform = "rotate-90";
          }
        ];

        clients = [
          {
            match.exe-regex = "^${pkgs.swaynotificationcenter}/.*";
            capabilities = [
              "layer-shell"
            ];
          }
          {
            match.exe-regex = "^${pkgs.waybar}/.*";
            capabilities = [
              "layer-shell"
              "workspace-manager"
            ];
          }
          {
            match.exe-regex = "^${pkgs.wayland-pipewire-idle-inhibit}/.*";
            capabilities = [
              "layer-shell"
            ];
          }
          {
            # warning: this allows unrestricted access to the clipboard with wl-copy and wl-paste
            # without these capabilities wl-copy just freezes and programs like neovim which require wl-copy/wl-paste for yanking/pasting to the global clipboard become unusable
            match.exe-regex = "^${pkgs.wl-clipboard}/.*";
            capabilities = [
              "data-control"
            ];
          }
        ];

        # lock after 10 minutes idle
        idle = {
          minutes = 10;
          # During the grace period, the screen goes black but the outputs are not yet disabled and the on-idle action does not yet run.
          # This is a visual indicator that the system will soon get idle.
          grace-period.seconds = 5;
        };
        on-idle = {
          type = "named";
          name = "lock";
        };

        focus-follows-mouse = true;
        workspace-display-order = "sorted";

        show-bar = false;
        show-titles = true;
        middle-click-paste = false;

        theme = with config.lib.stylix.colors.withHashtag; {
          bg-color = base00;
          font = config.stylix.fonts.monospace.name;

          border-width = 1;
          border-color = base03;

          # title-height = 1; # disable title bars
          title-font = config.stylix.fonts.monospace.name;
          attention-requested-bg-color = base09;
          captured-focused-title-bg-color = base08;
          captured-unfocused-title-bg-color = base0A;
          focused-inactive-title-bg-color = base03;
          focused-inactive-title-text-color = base05;
          focused-title-bg-color = base0D;
          focused-title-text-color = base00;
          separator-color = base03;
          unfocused-title-bg-color = base03;
          unfocused-title-text-color = base05;
          highlight-color = base0B;

          bar-position = "top";
          bar-height = 16;
          bar-separator-width = 1;
          bar-font = config.stylix.fonts.monospace.name;
          bar-bg-color = base01;
          bar-status-text-color = base05;
        };
      };
  };
}
