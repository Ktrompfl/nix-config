{
  actions,
  cfg,
  defaultKeys,
  hotkeyType,
  lib,
  mkEnumOption,
  pkgs,
}:
{
  enable = lib.mkEnableOption "Ninjabrain Bot";

  package = lib.mkPackageOption pkgs "ninjabrain-bot" { };

  theme = lib.mkOption {
    type = lib.types.bool;
    default = true;

    description = "Add the active scheme as a custom theme, and select it.";
  };

  themeName = lib.mkOption {
    type = lib.types.strMatching "[^.]*";
    default = "Stylix";
    description = "What the generated theme is called in the bot's theme list.";
  };

  hotkeys = lib.mkOption {
    default = { };
    description = "The bot's hotkeys.";
    type = lib.types.submodule {
      options = lib.mapAttrs (
        action: _:
        lib.mkOption {
          type = hotkeyType;
          default = {
            key = defaultKeys.${action};
          };
          defaultText = lib.literalExpression ''{ key = "${defaultKeys.${action}}"; }'';
          description = "Hotkey for the ${action} action.";
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
          default = if cfg.theme then -1 else 1;
          defaultText = lib.literalExpression "-1 with the theme, otherwise 1";
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
}
