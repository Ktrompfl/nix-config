{
  config,
  lib,
  pkgs,
  ...
}:
let
  # --- scripts ---

  # workaround for:
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
    ${lib.getExe' pkgs.imagemagick "magick"} "$TMPFILE" -crop "$region" +repage "$output"

    # --- copy to clipboard ---
    wl-copy --type "image/png" < "$output"

    # --- notify ---
    ${lib.getExe' pkgs.libnotify "notify-send"} --app-name="Screenshot" --urgency=normal --expire-time=3000 --icon="$output" "Screenshot Saved" "Screenshot saved to $output and copied to clipboard"
  '';

  # track the mode stack in a tmpfile and signal waybar on changes to update a
  # mode indicator until jay / waybar support this natively via ipc
  jay-mode =
    let
      signal = 1;
      socket = "$XDG_RUNTIME_DIR/waybar/custom/mode";
    in
    pkgs.writeShellScript "jay-mode" /* sh */ ''
      mkdir -p "$(dirname "${socket}")"

      if [[ $1 ]]; then
        echo "$1" >> "${socket}"
      else
        sed -i '$ d' "${socket}" 2>/dev/null
      fi

      pkill -RTMIN+${toString signal} waybar
    '';

  # --- action constructors ---

  mkExec = exec: {
    type = "exec";
    inherit exec;
  };

  mkMoveToOutput = ws: output: [
    {
      type = "move-to-workspace";
      name = ws;
    }
    {
      type = "move-to-output";
      workspace = ws;
      output.name = output;
    }
  ];

  mkPushMode = mode: {
    type = "multi";
    actions = [
      {
        type = "push-mode";
        name = mode;
      }
      (mkExec [
        "${jay-mode}"
        mode
      ])
    ];
  };

  popMode = {
    type = "multi";
    actions = [
      {
        type = "simple";
        cmd = "pop-mode";
      }
      (mkExec "${jay-mode}")
    ];
  };
in
{
  wayland.windowManager.jay = {
    enable = true;
    settings =
      let
        modifier = "logo";

        dirKeys = [
          {
            key = "h";
            arrow = "Left";
            dir = "left";
            field = "dx1";
            sign = -1;
          }
          {
            key = "j";
            arrow = "Down";
            dir = "down";
            field = "dy2";
            sign = 1;
          }
          {
            key = "k";
            arrow = "Up";
            dir = "up";
            field = "dy1";
            sign = -1;
          }
          {
            key = "l";
            arrow = "Right";
            dir = "right";
            field = "dx2";
            sign = 1;
          }
        ];

        # generate {prefix}{key} and {prefix}{arrow} bindings for all four directions
        dirBindings =
          prefix: mkAction:
          lib.listToAttrs (
            lib.concatMap (
              {
                key,
                arrow,
                dir,
                ...
              }:
              [
                (lib.nameValuePair "${prefix}${key}" (mkAction dir))
                (lib.nameValuePair "${prefix}${arrow}" (mkAction dir))
              ]
            ) dirKeys
          );

        vtBindings = lib.listToAttrs (
          lib.genList (
            n:
            lib.nameValuePair "ctrl-alt-F${toString (n + 1)}" {
              type = "switch-to-vt";
              num = n + 1;
            }
          ) 12
        );

        workspaceBindings = lib.listToAttrs (
          lib.concatMap (
            n:
            let
              ws = toString n;
            in
            [
              (lib.nameValuePair "${modifier}-${ws}" {
                type = "show-workspace";
                name = ws;
              })
              (lib.nameValuePair "${modifier}-shift-${ws}" {
                type = "move-to-workspace";
                name = ws;
              })
            ]
          ) (lib.range 0 9)
        );
      in
      {
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
          XCURSOR_THEME = "${config.home.pointerCursor.name}";
          XCURSOR_SIZE = "${toString config.home.pointerCursor.size}";

          # gtk theme
          GTK_THEME = "${config.gtk.theme.name}";

          # qt theme
          QT_QPA_PLATFORMTHEME = "${config.qt.platformTheme.name}";
          QT_STYLE_OVERRIDE = "${config.qt.style.name}";
        };

        # enable debug logging for unstable builds
        # log-level = "debug";

        # --- hardware ---

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
                xkb_geometry { include "pc(pc105)" };
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
                xkb_geometry { include "pc(pc104)"};
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
            match.name = "SmartCloud AL68 Keyboard";
            keymap.name = "external";
          }
          {
            match.name = "SmartCloud AL68 Keyboard Mouse";
            keymap.name = "external";
          }
          {
            match.name = "SmartCloud AL68 Keyboard Consumer Control";
            keymap.name = "external";
          }
          {
            match.name = "SmartCloud AL68 Keyboard System Control";
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
            x = 0;
            y = 0;
            mode = {
              width = 1920;
              height = 1080;
              refresh-rate = 60.049;
            };
          }
          {
            match.model = "EPSON PJ";
            name = "beamer";
            x = 0;
            y = 1080;
            mode = {
              width = 1920;
              height = 1080;
              refresh-rate = 60.0;
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

        # --- key bindings ---

        window-management-key = "Super_L"; # logo uses different symbol names

        actions = {
          lock = mkExec {
            prog = lib.getExe pkgs.swaylock;
            privileged = true;
          };
          poweroff = mkExec [
            "systemctl"
            "poweroff"
          ];
          reboot = mkExec [
            "systemctl"
            "reboot"
          ];
          suspend = mkExec [
            "systemctl"
            "suspend"
          ];
        };

        shortcuts =
          vtBindings
          // workspaceBindings
          // dirBindings "${modifier}-" (dir: "focus-${dir}")
          // dirBindings "${modifier}-shift-" (dir: "move-${dir}")
          // dirBindings "${modifier}-shift-ctrl-" (dir: {
            type = "move-to-output";
            direction = dir;
          })
          // {
            # compositor
            "${modifier}-shift-q" = "quit";
            "${modifier}-shift-r" = "reload-config-toml";

            # windows
            "${modifier}-q" = "close";
            "${modifier}-f" = "toggle-fullscreen";
            "${modifier}-space" = "toggle-floating";
            "${modifier}-n" = "toggle-mono";
            "${modifier}-v" = "toggle-split";
            "${modifier}-u" = "split-horizontal";
            "${modifier}-i" = "split-vertical";
            "${modifier}-Escape" = "disable-pointer-constraint";
            "${modifier}-t" = "show-titles";
            "${modifier}-shift-t" = "hide-titles";

            # focus
            "${modifier}-Tab" = "focus-next";
            "${modifier}-shift-Tab" = "focus-prev";
            "${modifier}-Delete" = "focus-tiles";
            "${modifier}-Prior" = "focus-above"; # layer above
            "${modifier}-Next" = "focus-below"; # layer below
            "${modifier}-g" = "focus-parent";
            "${modifier}-c" = "warp-mouse-to-focus";

            # marks — next key identifies the mark
            "${modifier}-y" = {
              type = "jump-to-mark";
            };
            "${modifier}-shift-y" = {
              type = "create-mark";
            };

            # modes
            "${modifier}-m" = mkPushMode "mirror";
            "${modifier}-p" = mkPushMode "system";
            "${modifier}-r" = mkPushMode "resize";

            # audio (wireplumber)
            XF86AudioRaiseVolume = mkExec [
              "wpctl"
              "set-volume"
              "@DEFAULT_AUDIO_SINK@"
              "5%+"
              "--limit"
              "1.5"
            ];
            XF86AudioLowerVolume = mkExec [
              "wpctl"
              "set-volume"
              "@DEFAULT_AUDIO_SINK@"
              "5%-"
              "--limit"
              "0.0"
            ];
            XF86AudioMute = mkExec [
              "wpctl"
              "set-mute"
              "@DEFAULT_AUDIO_SINK@"
              "toggle"
            ];
            XF86AudioMicMute = mkExec [
              "wpctl"
              "set-mute"
              "@DEFAULT_AUDIO_SOURCE@"
              "toggle"
            ];

            # player
            XF86AudioPlay = mkExec [
              (lib.getExe pkgs.playerctl)
              "play-pause"
            ];
            XF86AudioPause = mkExec [
              (lib.getExe pkgs.playerctl)
              "play-pause"
            ];
            XF86AudioNext = mkExec [
              (lib.getExe pkgs.playerctl)
              "next"
            ];
            XF86AudioPrev = mkExec [
              (lib.getExe pkgs.playerctl)
              "previous"
            ];
            XF86AudioStop = mkExec [
              (lib.getExe pkgs.playerctl)
              "stop"
            ];

            # screenshot
            "${modifier}-s" = mkExec [
              "${jay-screenshot}"
              "workspace"
            ];
            "${modifier}-shift-s" = mkExec [
              "${jay-screenshot}"
              "window"
            ];
            "${modifier}-ctrl-s" = mkExec [
              "${jay-screenshot}"
              "region"
            ];

            # launch
            "${modifier}-Return" = mkExec [
              (lib.getExe pkgs.app2unit)
              "footclient"
            ];
            "${modifier}-d" = mkExec "fuzzel";
            "${modifier}-a" = mkExec [
              "swaync-client"
              "-t"
            ];
          };

        modes = {
          mirror.shortcuts =
            let
              wl-present = lib.getExe' pkgs.wl-mirror "wl-present";
              present-action = action: [
                popMode
                (mkExec [
                  wl-present
                  action
                ])
              ];
            in
            {
              "${modifier}-m" = popMode;
              "Escape" = popMode;

              "m" = present-action "mirror";
              "c" = present-action "custom";
              "f" = present-action "toggle-freeze";
              "z" = present-action "freeze";
              "shift-z" = present-action "unfreeze";
              "o" = present-action "set-output";
              "r" = present-action "set-region";
              "shift-r" = present-action "unset-region";
              "s" = present-action "set-scaling";
            };

          resize.shortcuts =
            let
              amount = 10;
              mkResize = field: val: {
                type = "resize";
                ${field} = val;
                repeat = true;
              };
            in
            lib.listToAttrs (
              lib.concatMap (
                {
                  key,
                  arrow,
                  field,
                  sign,
                  ...
                }:
                [
                  (lib.nameValuePair key (mkResize field (sign * amount)))
                  (lib.nameValuePair arrow (mkResize field (sign * amount)))
                  (lib.nameValuePair "shift-${key}" (mkResize field (-(sign * amount))))
                  (lib.nameValuePair "shift-${arrow}" (mkResize field (-(sign * amount))))
                ]
              ) dirKeys
            )
            // {
              "${modifier}-r" = popMode;
              "Escape" = popMode;
            };

          system.shortcuts = {
            "${modifier}-p" = popMode;
            "Escape" = popMode;
            "l" = [
              popMode
              "$lock"
            ];
            "s" = [
              popMode
              "$poweroff"
            ];
            "r" = [
              popMode
              "$reboot"
            ];
            "h" = [
              popMode
              "$suspend"
            ];
          };
        };

        # --- window rules ---

        windows = [
          {
            name = "spotify";
            match.app-id = "spotify";
            action = mkMoveToOutput "5" "vertical";
          }
          {
            name = "vesktop";
            match.app-id = "vesktop";
            action = mkMoveToOutput "5" "vertical";
          }
          {
            name = "firefox";
            match.app-id = "firefox";
            action = mkMoveToOutput "1" "horizontal";
          }
          {
            name = "wl-mirror";
            match.app-id = "at.yrlf.wl_mirror";
            match.just-mapped = true;
            auto-focus = false;
            action = mkMoveToOutput "0" "beamer" ++ [
              {
                type = "simple";
                cmd = "enter-fullscreen";
              }
            ];
          }
        ];

        clients = [
          {
            match.exe-regex = "^${pkgs.swaynotificationcenter}/.*";
            capabilities = [ "layer-shell" ];
          }
          {
            match.exe-regex = "^${pkgs.waybar}/.*";
            capabilities = [
              # TODO: display the active window title via zwlr_foreign_toplevel_manager_v1
              # by combining parts of the waybar modules wlr/taskbar and sway/window
              "foreign-toplevel-manager"
              "layer-shell"
              "workspace-manager"
            ];
          }
          {
            match.exe-regex = "^${pkgs.wl-mirror}/.*";
            capabilities = [ "screencopy" ];
          }
          {
            match.exe-regex = "^${pkgs.wayland-pipewire-idle-inhibit}/.*";
            capabilities = [ "layer-shell" ];
          }
          {
            # warning: this allows unrestricted clipboard access via wl-copy/wl-paste
            # without it wl-copy freezes and neovim clipboard yanking breaks
            match.exe-regex = "^${pkgs.wl-clipboard}/.*";
            capabilities = [ "data-control" ];
          }
        ];

        # --- behavior ---

        focus-follows-mouse = true;
        unstable-mouse-follows-focus = true;
        fallback-output-mode = "focus"; # more useful with mouse-follows-focus
        workspace-display-order = "sorted";

        # lock after 10 minutes idle
        idle = {
          minutes = 10;
          # screen goes black during grace period before idle action and output disable
          grace-period.seconds = 5;
        };
        on-idle = "$lock";

        # --- appearance ---

        show-bar = false;
        show-titles = true;
        middle-click-paste = false;

        theme = with config.lib.stylix.colors.withHashtag; {
          bg-color = base00;
          font = config.stylix.fonts.monospace.name;

          border-width = 1;
          border-color = base03;

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

  preservation.preserveAt.state-dir.directories = [ ".local/share/jay" ];
}
