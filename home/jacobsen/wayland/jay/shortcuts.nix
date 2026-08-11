{
  config,
  jayLib,
  lib,
  pkgs,
  ...
}:
let
  inherit (jayLib)
    barInit
    exec
    setBlockText
    sh
    ;

  modifier = "logo";

  # Shortcuts that should fire again while the key is held. Only complex
  # shortcuts can repeat, so this wraps a plain shortcut table into one.
  repeating = lib.mapAttrs (
    _: action: {
      inherit action;
      repeat = true;
    }
  );

  # --- modes ---

  # jay owns the actual mode stack; the indicator only ever shows the mode
  # that was entered last, which is all the shortcuts below ever push.
  pushMode = name: [
    {
      type = "push-mode";
      inherit name;
    }
    (setBlockText "/mode" (lib.toUpper name))
  ];

  popMode = [
    "pop-mode"
    (setBlockText "/mode" "NORMAL")
  ];

  # --- keys ---

  # `field`/`sign` describe which edge of a window the direction resizes and
  # in which direction that edge grows, see the resize mode below.
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

  workspaceBindings = lib.listToAttrs (
    lib.concatMap (ws: [
      (lib.nameValuePair "${modifier}-${ws}" {
        type = "show-workspace";
        name = ws;
      })
      (lib.nameValuePair "${modifier}-shift-${ws}" {
        type = "move-to-workspace";
        name = ws;
      })
    ]) (map toString (lib.range 0 9))
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

  # --- screenshots ---

  # The toml config cannot compute geometries by itself, so they are read
  # back out of the compositor: the focused window knows its own position,
  # and its workspace knows both its position and the output it is on.
  screenshot = pkgs.writeShellApplication {
    name = "jay-screenshot";
    runtimeInputs = with pkgs; [
      config.wayland.windowManager.jay.package
      grim
      jq
      libnotify
      satty
    ];
    text = ''
      mode=''${1:-window}

      # `jay --json` writes one JSON object per matched node; slurping keeps
      # the first one and yields nothing at all when there was no match.
      query() {
        jay --json tree query "$@" | jq -s '.[0] // empty'
      }

      geometry() {
        jq -er '"\(.position.x1),\(.position.y1) \(.position.width)x\(.position.height)"' <<<"$1"
      }

      fail() {
        notify-send --urgency=critical "screenshot" "$1"
        exit 1
      }

      window=$(query match-windows -e 'focused = true')
      [[ -n $window ]] || fail "no focused window"

      case $mode in
        window)
          args=(-g "$(geometry "$window")")
          ;;
        workspace | output)
          name=$(jq -er '.workspace' <<<"$window") || fail "focused window is not on a workspace"
          workspace=$(query workspace-name "$name")
          [[ -n $workspace ]] || fail "workspace $name not found"
          if [[ $mode == workspace ]]; then
            args=(-g "$(geometry "$workspace")")
          else
            args=(-o "$(jq -er '.output' <<<"$workspace")")
          fi
          ;;
        *)
          fail "unknown mode: $mode"
          ;;
      esac

      grim "''${args[@]}" - | satty --filename -
    '';
  };

  screenshotOf =
    mode:
    exec [
      (lib.getExe screenshot)
      mode
    ];
in
{
  # Everything that is bound to a key, including the input modes.
  wayland.windowManager.jay.settings = {
    window-management-key = "Super_L"; # logo uses different symbol names

    # Everything that is worth holding down rather than tapping: navigating
    # focus, dragging a window or workspace along, and stepping the volume.
    # Repeating is the only reason these are complex shortcuts.
    complex-shortcuts = repeating (
      dirBindings "${modifier}-" (dir: "focus-${dir}")
      // dirBindings "${modifier}-shift-" (dir: "move-${dir}")
      // dirBindings "${modifier}-shift-ctrl-" (dir: {
        type = "move-to-output";
        direction = dir;
      })
      // {
        # focus
        "${modifier}-Tab" = "focus-next";
        "${modifier}-shift-Tab" = "focus-prev";
        "${modifier}-Prior" = "focus-above"; # layer above
        "${modifier}-Next" = "focus-below"; # layer below
        "${modifier}-g" = "focus-parent";

        # audio (wireplumber)
        XF86AudioRaiseVolume = exec [
          "wpctl"
          "set-volume"
          "@DEFAULT_AUDIO_SINK@"
          "5%+"
          "--limit"
          "1.5"
        ];
        XF86AudioLowerVolume = exec [
          "wpctl"
          "set-volume"
          "@DEFAULT_AUDIO_SINK@"
          "5%-"
          "--limit"
          "0.0"
        ];
      }
    );

    shortcuts =
      vtBindings
      // workspaceBindings
      // {
        # compositor
        "${modifier}-shift-q" = "quit";
        # a reload restarts the status program, losing the pushed blocks
        "${modifier}-shift-r" = [
          "reload-config-toml"
          (exec (lib.getExe barInit))
        ];

        # windows
        "${modifier}-q" = "close";
        "${modifier}-f" = "toggle-fullscreen";
        "${modifier}-space" = "toggle-floating";
        "${modifier}-n" = "toggle-mono";
        "${modifier}-v" = "toggle-split";
        "${modifier}-y" = "tile-major";
        "${modifier}-shift-y" = "split-major";
        "${modifier}-u" = "split-horizontal";
        "${modifier}-i" = "split-vertical";
        "${modifier}-Escape" = "disable-pointer-constraint";
        "${modifier}-t" = "show-titles";
        "${modifier}-shift-t" = "hide-titles";

        # focus
        "${modifier}-Delete" = "focus-tiles";
        "${modifier}-c" = "warp-mouse-to-focus";

        # modes
        "${modifier}-m" = pushMode "mirror";
        "${modifier}-p" = pushMode "system";
        "${modifier}-r" = pushMode "resize";

        # audio (wireplumber)
        XF86AudioMute = exec [
          "wpctl"
          "set-mute"
          "@DEFAULT_AUDIO_SINK@"
          "toggle"
        ];
        XF86AudioMicMute = exec [
          "wpctl"
          "set-mute"
          "@DEFAULT_AUDIO_SOURCE@"
          "toggle"
        ];

        # player
        XF86AudioPlay = exec [
          (lib.getExe pkgs.playerctl)
          "play-pause"
        ];
        XF86AudioPause = exec [
          (lib.getExe pkgs.playerctl)
          "play-pause"
        ];
        XF86AudioNext = exec [
          (lib.getExe pkgs.playerctl)
          "next"
        ];
        XF86AudioPrev = exec [
          (lib.getExe pkgs.playerctl)
          "previous"
        ];
        XF86AudioStop = exec [
          (lib.getExe pkgs.playerctl)
          "stop"
        ];

        # screenshot
        "${modifier}-s" = screenshotOf "output";
        "${modifier}-shift-s" = screenshotOf "window";
        "${modifier}-ctrl-s" = screenshotOf "workspace";

        # launch
        "${modifier}-Return" = exec [
          "app2unit"
          "footclient"
        ];
        "${modifier}-d" = exec "fuzzel";
        "${modifier}-a" = exec [
          "swaync-client"
          "-t"
        ];
        "${modifier}-shift-v" =
          sh "cliphist list | fuzzel --dmenu --with-nth 2 | cliphist decode | wl-copy";
      };

    # Modes inherit the top-level shortcuts, so they only add to them.
    modes = {
      mirror.shortcuts =
        let
          present =
            action:
            popMode
            ++ [
              (exec [
                (lib.getExe' pkgs.wl-mirror "wl-present")
                action
              ])
            ];
        in
        {
          "${modifier}-m" = popMode;
          Escape = popMode;

          m = present "mirror";
          c = present "custom";
          f = present "toggle-freeze";
          z = present "freeze";
          "shift-z" = present "unfreeze";
          o = present "set-output";
          r = present "set-region";
          "shift-r" = present "unset-region";
          s = present "set-scaling";
        };

      resize = {
        shortcuts = {
          "${modifier}-r" = popMode;
          Escape = popMode;
        };

        complex-shortcuts = repeating (
          let
            amount = 10;
            resize = field: val: {
              type = "resize";
              ${field} = val;
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
              let
                grow = resize field (sign * amount);
                shrink = resize field (-(sign * amount));
              in
              [
                (lib.nameValuePair key grow)
                (lib.nameValuePair arrow grow)
                (lib.nameValuePair "shift-${key}" shrink)
                (lib.nameValuePair "shift-${arrow}" shrink)
              ]
            ) dirKeys
          )
        );
      };

      system.shortcuts = {
        "${modifier}-p" = popMode;
        Escape = popMode;

        l = popMode ++ [ "$lock" ];
        s = popMode ++ [
          (exec [
            "systemctl"
            "poweroff"
          ])
        ];
        r = popMode ++ [
          (exec [
            "systemctl"
            "reboot"
          ])
        ];
        h = popMode ++ [ "$suspend" ];
        i = popMode ++ [ "$toggle-idle-inhibitor" ];
      };
    };
  };
}
