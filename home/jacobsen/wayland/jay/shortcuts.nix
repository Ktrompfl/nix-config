{
  jayLib,
  lib,
  pkgs,
  ...
}:
let
  inherit (jayLib)
    bar
    exec
    moveToOutput
    moveToWorkspace
    pushMode
    resize
    showWorkspace
    switchToVt
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
  popMode = [
    "pop-mode"
    (bar.mode "normal")
  ];

  # A mode entry that leaves the mode before it runs.
  leave = action: popMode ++ [ action ];

  # Builds the `modes` table and the top-level shortcuts that enter them. A
  # mode inherits the top-level shortcuts, so it only adds to them, and it is
  # always left again by its own key and by Escape.
  mkModes =
    modes:
    let
      mkMode =
        {
          name,
          key,
          shortcuts ? { },
          complexShortcuts ? { },
        }:
        {
          enter.${key} = [
            (pushMode name)
            (bar.mode name)
          ];

          mode.${name} = {
            shortcuts = {
              ${key} = popMode;
              Escape = popMode;
            }
            // shortcuts;
          }
          // lib.optionalAttrs (complexShortcuts != { }) {
            complex-shortcuts = complexShortcuts;
          };
        };
      built = map mkMode modes;
    in
    {
      shortcuts = lib.mergeAttrsList (map (m: m.enter) built);
      modes = lib.mergeAttrsList (map (m: m.mode) built);
    };

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
      (lib.nameValuePair "${modifier}-${ws}" (showWorkspace ws))
      (lib.nameValuePair "${modifier}-shift-${ws}" (moveToWorkspace ws))
    ]) (map toString (lib.range 0 9))
  );

  vtBindings = lib.listToAttrs (
    lib.genList (n: lib.nameValuePair "ctrl-alt-F${toString (n + 1)}" (switchToVt (n + 1))) 12
  );

  # --- programs ---

  # `output` and `workspace` need a focused window to derive their geometry
  # from, see ../../../../pkgs/jay-scripts. The shared library configuration
  # computes the geometry itself and can capture an empty one.
  screenshotOf =
    mode:
    exec [
      (lib.getExe pkgs.jay-screenshot)
      mode
    ];

  present =
    action:
    leave (exec [
      (lib.getExe' pkgs.wl-mirror "wl-present")
      action
    ]);

  modeConfig = mkModes [
    {
      name = "mirror";
      key = "${modifier}-m";
      shortcuts = {
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
    }
    {
      name = "resize";
      key = "${modifier}-r";
      complexShortcuts = repeating (
        let
          amount = 10;
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
              grow = resize { ${field} = sign * amount; };
              shrink = resize { ${field} = -(sign * amount); };
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
    }
    {
      name = "system";
      key = "${modifier}-p";
      shortcuts = {
        l = leave "$lock";
        s = leave (exec [
          "systemctl"
          "poweroff"
        ]);
        r = leave (exec [
          "systemctl"
          "reboot"
        ]);
        h = leave "$suspend";
        i = leave "$toggle-idle-inhibitor";
      };
    }
  ];
in
{
  # Everything that is bound to a key, including the input modes.

  # Everything that is worth holding down rather than tapping: navigating
  # focus, dragging a window or workspace along, and stepping the volume.
  # Repeating is the only reason these are complex shortcuts.
  complex-shortcuts = repeating (
    dirBindings "${modifier}-" (dir: "focus-${dir}")
    // dirBindings "${modifier}-shift-" (dir: "move-${dir}")
    // dirBindings "${modifier}-shift-ctrl-" (dir: moveToOutput { direction = dir; })
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
    // modeConfig.shortcuts
    // {
      # compositor
      "${modifier}-shift-q" = "quit";
      # a reload restarts the status program, losing the pushed blocks
      "${modifier}-shift-r" = [
        "reload-config-toml"
        bar.init
      ];

      # windows
      "${modifier}-q" = "close";
      "${modifier}-f" = "toggle-fullscreen";
      "${modifier}-space" = "toggle-floating";
      "${modifier}-n" = {
        type = "toggle-mono";
        target = "auto";
      };
      "${modifier}-v" = {
        type = "toggle-split";
        target = "auto";
      };
      "${modifier}-b" = "split-major";
      "${modifier}-Escape" = "disable-pointer-constraint";
      "${modifier}-t" = "show-titles";
      "${modifier}-shift-t" = "hide-titles";

      "${modifier}-y" = "tile-major";
      "${modifier}-shift-y" = "split-major";

      # focus
      "${modifier}-Delete" = "focus-tiles";
      "${modifier}-c" = "warp-mouse-to-focus";

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
        "runapp"
        "footclient"
      ];
      "${modifier}-shift-Return" = exec [
        "runapp"
        "foot"
      ];
      "${modifier}-d" = exec "fuzzel";
      "${modifier}-a" = exec [
        "swaync-client"
        "-t"
      ];
      "${modifier}-shift-v" = exec (lib.getExe pkgs.jay-clipboard-history);
    };

  inherit (modeConfig) modes;
}
