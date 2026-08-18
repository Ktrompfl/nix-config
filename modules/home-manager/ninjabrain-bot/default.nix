# Ninjabrain Bot with its settings declared here rather than clicked together
# in its GUI. On Wayland the bot cannot read the clipboard or see a hotkey, so
# `out-of-the-box` wraps it in `nbb-ootb`, which gives it an X server of its own
# and replays the hotkeys below into it; see ../../../pkgs/nbb-ootb.
#
# based on https://tangled.org/althaea.zone/ninjabrain-bot-nix/
{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.programs.ninjabrain-bot;

  inherit (import ./keys.nix)
    actions
    keys
    locations
    modifiers
    ;

  # The bot stores a choice as an index, so enums are listed in its own order.
  mkEnumOption =
    values: default:
    lib.mkOption {
      inherit default;
      type = lib.types.enum values;
      apply = value: lib.lists.findFirstIndex (candidate: candidate == value) 0 values;
      description = "One of ${lib.concatStringsSep ", " values}.";
    };

  hotkeyType = lib.types.submodule (
    { config, ... }:
    {
      options = {
        key = lib.mkOption {
          type = lib.types.nullOr (lib.types.enum (lib.attrNames keys));
          example = "F9";
          description = "Which key fires this action, or null to leave it unbound.";
        };

        modifiers = lib.mkOption {
          type = lib.types.listOf (lib.types.enum (lib.attrNames modifiers));
          default = [ ];
          example = [ "CTRL_L" ];
          description = "Modifiers held while the key is pressed.";
        };

        location = lib.mkOption {
          type = lib.types.enum (lib.attrNames locations);
          default = "STANDARD";
          description = "Which of several same-named keys this is.";
        };

        code = lib.mkOption {
          type = lib.types.nullOr lib.types.int;
          default = if config.key == null then null else keys.${config.key};
          defaultText = lib.literalMD "derived from `key`";
          description = "The value the bot expects from JNativeHook for this key.";
        };

        keycode = lib.mkOption {
          type = lib.types.nullOr lib.types.int;
          default = if config.key == null then null else keys.${config.key} + 8;
          defaultText = lib.literalMD "derived from `key`";
          description = "The X keycode to inject to produce it.";
        };
      };
    }
  );

  defaultKeys = {
    increment = "F7";
    decrement = "F8";
    reset = "F9";
    undo = "F10";
    redo = "F12";
    minimize = "minus";
    alt-std = "equal";
    lock = "F4";
    boat = "F6";
    mod-360 = "bracketleft";
    aa-mode = "bracketright";
  };

  hotkeys = lib.removeAttrs cfg.hotkeys [ "_module" ];

  hotkeyEntries = lib.concatMapAttrs (
    action: hotkey:
    lib.optionalAttrs (hotkey.code != null) {
      "${actions.${action}}_code" = hotkey.code + locations.${hotkey.location} * 65536;
      "${actions.${action}}_modifier" = lib.foldl' (
        mask: modifier: lib.bitOr mask modifiers.${modifier}.mask
      ) 0 hotkey.modifiers;
    }
  ) hotkeys;

  actionKeys = lib.concatMapAttrs (
    action: hotkey:
    lib.optionalAttrs (hotkey.keycode != null) {
      ${action} = {
        inherit (hotkey) keycode;
        modifiers = map (modifier: modifiers.${modifier}.keycode) hotkey.modifiers;
      };
    }
  ) hotkeys;

  settings = lib.filterAttrs (_: value: value != null) (
    lib.removeAttrs cfg.settings [ "_module" ] // hotkeyEntries
  );

  palette = lib.genAttrs (map (digit: "base0${digit}") (lib.stringToCharacters "0123456789ABCDEF")) (
    name: config.lib.stylix.colors.${name}
  );

  json = (pkgs.formats.json { }).generate;

  prefsArgs = [
    "--settings"
    "${json "ninjabrain-bot-settings.json" settings}"
  ]
  ++ lib.optionals cfg.stylix [
    "--colors"
    "${json "ninjabrain-bot-palette.json" palette}"
    "--theme-name"
    cfg.themeName
  ];

  prefs = pkgs.runCommand "ninjabrain-bot-prefs.xml" { } ''
    ${lib.getExe pkgs.python3} ${./prefs.py} ${lib.escapeShellArgs prefsArgs} > $out
  '';
in
{
  options.programs.ninjabrain-bot = {
    enable = lib.mkEnableOption "Ninjabrain Bot";

    package = lib.mkPackageOption pkgs "ninjabrain-bot" { };

    out-of-the-box = lib.mkOption {
      type = lib.types.bool;
      default = true;
      description = ''
        Wrap the bot in `nbb-ootb`, which runs it in an X server of its own and
        can replay its hotkeys. Without it the bot is installed as it comes,
        which under Wayland leaves it unable to read the clipboard or see a
        hotkey.
      '';
    };

    wrapperPackage = lib.mkPackageOption pkgs "nbb-ootb" { };

    finalPackage = lib.mkOption {
      type = lib.types.package;
      readOnly = true;
      description = "The bot as configured, wrapped or not.";
    };

    stylix = lib.mkOption {
      type = lib.types.bool;
      default = config.stylix.enable or false;
      defaultText = lib.literalExpression "config.stylix.enable";
      description = "Add the stylix palette as a custom theme, and select it.";
    };

    themeName = lib.mkOption {
      type = lib.types.strMatching "[^.]*";
      default = "Stylix";
      description = "What the generated theme is called in the bot's theme list.";
    };

    display = lib.mkOption {
      type = lib.types.str;
      default = ":77";
      description = "The X display the box runs on.";
    };

    geometry = lib.mkOption {
      type = lib.types.str;
      default = "480x320";
      description = "Initial size of the box. Only honoured while it floats.";
    };

    hotkeys = lib.mkOption {
      default = { };
      description = "The bot's hotkeys, and so also the actions `nbb-ootb` sends.";
      type = lib.types.submodule {
        options = lib.mapAttrs (
          action: _:
          lib.mkOption {
            type = hotkeyType;
            default = {
              key = defaultKeys.${action};
            };
            defaultText = lib.literalExpression ''{ key = "${defaultKeys.${action}}"; }'';
            description = "Hotkey for `nbb-ootb ${action}`.";
          }
        ) actions;
      };
    };

    settings = lib.mkOption {
      default = { };
      description = ''
        Preferences, under the names the bot stores them by. Undeclared names
        are passed through.
      '';
      type = lib.types.submodule {
        freeformType =
          with lib.types;
          attrsOf (
            nullOr (oneOf [
              bool
              int
              float
              str
            ])
          );

        options = {
          theme = lib.mkOption {
            type = lib.types.int;
            default = if cfg.stylix then -1 else 1;
            defaultText = lib.literalExpression "-1 with stylix, otherwise 1";
            description = ''
              Zero and up are the bot's own themes, -1 the first custom one
              and so on. Stylix is always first, so -1 selects it.
            '';
          };

          custom_themes = lib.mkOption {
            type = lib.types.str;
            default = "";
            description = ''
              Extra custom themes, "."-separated and already encoded, as built
              in the GUI or by `prefs.py --colors - --theme-only`.
            '';
          };

          custom_themes_names = lib.mkOption {
            type = lib.types.str;
            default = "";
            description = ''Names for `custom_themes`, "."-separated.'';
          };

          settings_version = lib.mkOption {
            type = lib.types.ints.positive;
            default = 3;
            visible = false;
            description = "Which settings layout this file is in.";
          };

          window_x = lib.mkOption {
            type = lib.types.int;
            default = 0;
            description = "Window position within the X server it runs in.";
          };

          window_y = lib.mkOption {
            type = lib.types.int;
            default = 0;
            description = "Window position within the X server it runs in.";
          };

          size = mkEnumOption [ "small" "medium" "large" ] "small";
          view = mkEnumOption [ "basic" "detailed" ] "basic";
          mc_version = mkEnumOption [ "pre_119" "post_119" ] "pre_119";
          stronghold_display_type = mkEnumOption [ "fourfour" "eighteight" "chunk" ] "fourfour";
          aa_toggle_type = mkEnumOption [ "automatic" "hotkey" ] "automatic";
          default_boat_type = mkEnumOption [ "gray" "blue" "green" ] "gray";
          angle_adjustment_type = mkEnumOption [ "subpixel" "tall" "custom" ] "subpixel";
          angle_adjustment_display_type = mkEnumOption [ "angle_change" "increments" ] "angle_change";

          sensitivity = lib.mkOption {
            type = lib.types.numbers.between 0 1;
            default = 0.012727597;
            description = "In game mouse sensitivity.";
          };

          sensitivity_manual = lib.mkOption {
            type = lib.types.numbers.between 0 1;
            default = 0.4341732;
          };

          sigma = lib.mkOption {
            type = lib.types.numbers.between 0.001 1;
            default = 0.1;
            description = "Assumed standard deviation of a measurement.";
          };

          sigma_alt = lib.mkOption {
            type = lib.types.numbers.between 0.001 1;
            default = 0.1;
          };

          sigma_manual = lib.mkOption {
            type = lib.types.numbers.between 0.001 1;
            default = 0.03;
          };

          sigma_boat = lib.mkOption {
            type = lib.types.numbers.between 0.0001 1;
            default = 0.001;
          };

          resolution_height = lib.mkOption {
            type = lib.types.numbers.between 1 16384;
            default = 16384;
            description = "Height the game renders at, for subpixel adjustment.";
          };

          boat_error = lib.mkOption {
            type = lib.types.numbers.between 0 0.7;
            default = 0.03;
          };

          overlay_hide_delay = lib.mkOption {
            type = lib.types.numbers.between 1 3600;
            default = 30.0;
          };

          custom_adjustment = lib.mkOption {
            type = lib.types.numbers.between 0 1;
            default = 0.01;
          };

          crosshair_correction = lib.mkOption {
            type = lib.types.numbers.between (-1) 1;
            default = 0;
          };

          language_v2 = lib.mkOption {
            type = lib.types.str;
            default = "";
            example = "en-US";
            description = "Language code, or empty for the system language.";
          };

          check_for_updates = lib.mkOption {
            type = lib.types.bool;
            default = false;
          };

          translucent = lib.mkOption {
            type = lib.types.bool;
            default = false;
          };

          always_on_top = lib.mkOption {
            type = lib.types.bool;
            default = true;
          };

          show_nether_coords = lib.mkOption {
            type = lib.types.bool;
            default = true;
          };

          show_angle_updates = lib.mkOption {
            type = lib.types.bool;
            default = false;
          };

          show_angle_errors = lib.mkOption {
            type = lib.types.bool;
            default = false;
          };

          auto_reset = lib.mkOption {
            type = lib.types.bool;
            default = false;
            description = "Reset after fifteen idle minutes.";
          };

          auto_reset_on_instance_change = lib.mkOption {
            type = lib.types.bool;
            default = false;
          };

          use_adv_statistics = lib.mkOption {
            type = lib.types.bool;
            default = true;
          };

          alt_clipboard_reader = lib.mkOption {
            type = lib.types.bool;
            default = false;
            description = "Read the clipboard on a hotkey rather than polling it.";
          };

          use_alt_std = lib.mkOption {
            type = lib.types.bool;
            default = false;
          };

          color_negative_coords = lib.mkOption {
            type = lib.types.bool;
            default = false;
          };

          use_precise_angle = lib.mkOption {
            type = lib.types.bool;
            default = false;
          };

          use_obs_overlay = lib.mkOption {
            type = lib.types.bool;
            default = false;
          };

          overlay_auto_hide = lib.mkOption {
            type = lib.types.bool;
            default = false;
          };

          overlay_lock_hide = lib.mkOption {
            type = lib.types.bool;
            default = false;
          };

          save_state = lib.mkOption {
            type = lib.types.bool;
            default = true;
          };

          enable_http_server = lib.mkOption {
            type = lib.types.bool;
            default = false;
            description = "Serve the calculator's state over HTTP.";
          };

          all_advancements = lib.mkOption {
            type = lib.types.bool;
            default = false;
          };

          one_dot_twenty_plus_aa = lib.mkOption {
            type = lib.types.bool;
            default = false;
          };

          mismeasure_warning_enabled = lib.mkOption {
            type = lib.types.bool;
            default = false;
          };

          direction_help_enabled = lib.mkOption {
            type = lib.types.bool;
            default = false;
          };

          combined_offset_information_enabled = lib.mkOption {
            type = lib.types.bool;
            default = true;
          };

          portal_linking_warning_enabled = lib.mkOption {
            type = lib.types.bool;
            default = true;
          };
        };
      };
    };
  };

  config = lib.mkIf cfg.enable {
    assertions = [
      {
        assertion = cfg.stylix -> config ? lib.stylix;
        message = "programs.ninjabrain-bot.stylix is set but stylix is not available.";
      }
    ]
    ++ lib.mapAttrsToList (action: hotkey: {
      assertion = (hotkey.code == null) == (hotkey.keycode == null);
      message = "programs.ninjabrain-bot.hotkeys.${action}: set both code and keycode, or neither.";
    }) hotkeys;

    programs.ninjabrain-bot.finalPackage =
      if cfg.out-of-the-box then
        cfg.wrapperPackage.override {
          inherit (cfg) display geometry;
          actions = actionKeys;
          ninjabrain-bot = cfg.package;
        }
      else
        cfg.package;

    # A copy, not the usual link into the store: the bot rewrites this file
    # from its own GUI, so it has to be writable. Those changes then last until
    # the next activation puts the declared settings back.
    home.activation.ninjabrainBotPrefs = lib.hm.dag.entryAfter [ "linkGeneration" ] ''
      run install -Dm644 ${prefs} ${config.home.homeDirectory}/.java/.userPrefs/ninjabrainbot/prefs.xml
    '';

    home.packages = [ cfg.finalPackage ];
  };
}
