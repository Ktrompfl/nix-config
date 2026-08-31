{
  config,
  lib,
  pkgs,
  ...
}:
let
  jayLib = rec {
    # --- action constructors ---
    #
    # One per action type the configuration uses; see the `Action` section of
    # jay's toml spec for the full list. Adding another one is a one-line
    # change and keeps the call sites free of `type = ...` tables.

    # `command` is a program name, an argument vector, or a `{ prog, args }`
    # table.
    exec = command: {
      type = "exec";
      exec = command;
    };

    # `connector` is a connector configuration, i.e. a `match` plus whatever
    # it changes about every output that matches it.
    configureConnector = connector: {
      type = "configure-connector";
      inherit connector;
    };
    enableConnector =
      match:
      configureConnector {
        inherit match;
        enabled = true;
      };
    disableConnector =
      match:
      configureConnector {
        inherit match;
        enabled = false;
      };

    configureIdle = idle: {
      type = "configure-idle";
      inherit idle;
    };

    defineAction = name: action: {
      type = "define-action";
      inherit name action;
    };

    pushMode = name: {
      type = "push-mode";
      inherit name;
    };

    showWorkspace = name: {
      type = "show-workspace";
      inherit name;
    };

    moveToWorkspace = name: {
      type = "move-to-workspace";
      inherit name;
    };

    # `target` picks the output, either relative to the current one
    # (`{ direction = "left"; }`) or by name (`{ output.name = "beamer"; }`).
    moveToOutput = target: { type = "move-to-output"; } // target;

    switchToVt = num: {
      type = "switch-to-vt";
      inherit num;
    };

    # `delta` moves one or more window edges, e.g. `{ dx1 = -10; }`.
    resize = delta: { type = "resize"; } // delta;

    # --- bar ---
    #
    # The two bar segments that reflect compositor state - the input mode and
    # the idle inhibitor - cannot be observed from the outside, so whatever
    # changes either of them pushes the new value into the bar itself.
    bar =
      let
        jay-bar = lib.getExe pkgs.jay-bar;
      in
      {
        mode =
          name:
          exec [
            jay-bar
            "mode"
            name
          ];
        idleInhibitor =
          state:
          exec [
            jay-bar
            "idle-inhibitor"
            state
          ];
        init = exec [
          jay-bar
          "init"
        ];
      };

    # --- constants ---

    # Shared by behavior.nix, which arms the timeout, and actions.nix, which
    # restores it when the idle inhibitor is switched off again.
    idle = {
      minutes = 10;
      # screen goes black during grace period before idle action and output disable
      grace-period.seconds = 15;
    };
  };

  # The settings are split into the same parts as the shared library
  # configuration in ../../../../pkgs/jay-config-lib/src, so that the two can
  # be read side by side: bar.nix corresponds to its bar.rs, and so on.
  # Wherever one side cannot do what the other does, the comment on both sides
  # says so.
  #
  # Each part is a plain expression that returns its piece of `settings`, not
  # a module, so nothing merges them for us: `mergeDisjoint` below is what
  # keeps two parts from quietly claiming the same key.
  settings =
    map
      (
        part:
        import part {
          inherit
            config
            jayLib
            lib
            pkgs
            ;
        }
      )
      [
        ./actions.nix
        ./bar.nix
        ./behavior.nix
        ./clients.nix
        ./inputs.nix
        ./outputs.nix
        ./shortcuts.nix
        ./theme.nix
        ./windows.nix
      ];

  mergeDisjoint =
    parts:
    let
      shared = lib.filter (name: lib.count (part: part ? ${name}) parts > 1) (
        lib.unique (lib.concatMap lib.attrNames parts)
      );
    in
    if shared == [ ] then
      lib.mergeAttrsList parts
    else
      throw "jay config: ${lib.concatStringsSep ", " shared} defined by more than one part";

  # The sixteen base16 slots of the active theme scheme, lower-cased because
  # that is how the shared library configuration reads them back.
  base16 = lib.mapAttrs' (name: lib.nameValuePair (lib.toLower name)) (
    lib.filterAttrs (
      name: _: builtins.match "base0[0-9A-F]" name != null
    ) config.theme.colors.withoutHashtag
  );
in
{
  config.packages = with pkgs; [
    jay

    runapp

    # The programs the two configurations share. The toml side refers to them
    # by store path, the shared library side needs them on `PATH`.
    jay-bar
    jay-clipboard-history
    jay-screenshot

    # extra programs used in the jay config
    playerctl
    wl-mirror
  ];

  config.xdg.config.files = {
    # config.so takes precendence over config.toml
    "jay/config.so".source = "${pkgs.jay-config-lib}/lib/config.so";

    "jay/config.toml" = {
      generator = (pkgs.formats.toml { }).generate "jay-config.toml";
      value = mergeDisjoint settings;
    };

    "jay/theme.toml".text = lib.concatStrings (
      lib.mapAttrsToList (name: value: "${name} = ${builtins.toJSON (toString value)}\n") (
        base16
        // {
          monospace_font = config.theme.fonts.monospace.name;
        }
      )
    );
  };

  # persist logs and session management
  config.preservation.preserveAt.state-dir.directories = [ ".local/share/jay" ];
}
