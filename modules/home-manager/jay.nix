{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (lib)
    literalExpression
    mkIf
    mkOption
    types
    ;

  cfg = config.wayland.windowManager.jay;
  tomlFormat = pkgs.formats.toml { };
in
{
  options.wayland.windowManager.jay = {
    enable = lib.mkEnableOption "Jay, a tiling wayland compositor";

    package = lib.mkPackageOption pkgs "jay" { };

    # This option is currently not used but the home-manager module tests for way-displays
    # expect this to be present for all entries of wayland.windowManager.
    systemd = {
      enable = lib.mkEnableOption null // {
        default = false;
        description = "";
      };

      variables = mkOption {
        type = types.listOf types.str;
        default = [ ];
        example = [ "--all" ];
        description = "";
      };

      extraCommands = mkOption {
        type = types.listOf types.str;
        default = [ ];
        description = "";
      };
    };

    library = mkOption {
      type = types.nullOr types.package;
      default = null;
      example = literalExpression ''
        craneLib.buildPackage {
          pname = "my-jay-config";
          src = ./my-jay-config;
          # Reuse jay-config's own build artifacts instead of recompiling it from scratch.
          cargoArtifacts = jay.packages.''${pkgs.system}.jay-config;
        }
      '';
      description = ''
        For users who need programmatic configuration beyond what TOML offers, Jay also supports
        configuration via a compiled Rust shared library using the jay-config crate. This is an
        advanced option -- the TOML config in `settings` is sufficient for the vast majority of
        use cases.

        This option expects a package that builds such a shared library (a crate with
        `crate-type = ["cdylib"]` and jay-config as a dependency) and places it at
        `$out/lib/config.so`. It is installed at ~/.config/jay/config.so.

        Jay loads config.so in preference to config.toml, so when this is set, `settings` is
        ignored by jay even though this module still writes it to disk if non-empty.
      '';
    };

    settings = mkOption {
      type = tomlFormat.type;
      default = { };
      example = literalExpression ''
        {
          # The keymap that is used for shortcuts and also sent to clients.
          keymap = \'\'
            xkb_keymap {
                xkb_keycodes { include "evdev+aliases(qwerty)" };
                xkb_types    { include "complete"              };
                xkb_compat   { include "complete"              };
                xkb_symbols  { include "pc+us+inet(evdev)"     };
            };
          \'\';

          # An action that will be executed when the GPU has been initialized.
          on-graphics-initialized = [
            { type = "exec"; exec = "mako"; }
            { type = "exec"; exec = "wl-tray-bridge"; }
          ];

          # Shortcuts that are processed by the compositor.
          shortcuts = {
            # Focus actions
            "alt-h" = "focus-left";
            "alt-j" = "focus-down";
            "alt-k" = "focus-up";
            "alt-l" = "focus-right";

            # Move actions
            "alt-shift-h" = "move-left";
            "alt-shift-j" = "move-down";
            "alt-shift-k" = "move-up";
            "alt-shift-l" = "move-right";

            # Split actions
            "alt-d" = "split-horizontal";
            "alt-v" = "split-vertical";

            # Toggle actions
            "alt-t" = "toggle-split";
            "alt-m" = "toggle-mono";
            "alt-u" = "toggle-fullscreen";

            # Parent/focus/close/floating
            "alt-f" = "focus-parent";
            "alt-c" = "open-control-center";
            "alt-shift-c" = "close";
            "alt-shift-f" = "toggle-floating";

            # Exec actions
            "Super_L" = { type = "exec"; exec = "alacritty"; };
            "alt-p"    = { type = "exec"; exec = "bemenu-run"; };

            # Quit and reload
            "alt-q"        = "quit";
            "alt-shift-r"  = "reload-config-toml";

            # Switch to VT
            "ctrl-alt-F1"  = { type = "switch-to-vt"; num = 1; };
            "ctrl-alt-F2"  = { type = "switch-to-vt"; num = 2; };
            "ctrl-alt-F3"  = { type = "switch-to-vt"; num = 3; };
            "ctrl-alt-F4"  = { type = "switch-to-vt"; num = 4; };
            "ctrl-alt-F5"  = { type = "switch-to-vt"; num = 5; };
            "ctrl-alt-F6"  = { type = "switch-to-vt"; num = 6; };
            "ctrl-alt-F7"  = { type = "switch-to-vt"; num = 7; };
            "ctrl-alt-F8"  = { type = "switch-to-vt"; num = 8; };
            "ctrl-alt-F9"  = { type = "switch-to-vt"; num = 9; };
            "ctrl-alt-F10" = { type = "switch-to-vt"; num = 10; };
            "ctrl-alt-F11" = { type = "switch-to-vt"; num = 11; };
            "ctrl-alt-F12" = { type = "switch-to-vt"; num = 12; };

            # Show workspace
            "alt-F1"  = { type = "show-workspace"; name = "1"; };
            "alt-F2"  = { type = "show-workspace"; name = "2"; };
            "alt-F3"  = { type = "show-workspace"; name = "3"; };
            "alt-F4"  = { type = "show-workspace"; name = "4"; };
            "alt-F5"  = { type = "show-workspace"; name = "5"; };
            "alt-F6"  = { type = "show-workspace"; name = "6"; };
            "alt-F7"  = { type = "show-workspace"; name = "7"; };
            "alt-F8"  = { type = "show-workspace"; name = "8"; };
            "alt-F9"  = { type = "show-workspace"; name = "9"; };
            "alt-F10" = { type = "show-workspace"; name = "10"; };
            "alt-F11" = { type = "show-workspace"; name = "11"; };
            "alt-F12" = { type = "show-workspace"; name = "12"; };

            # Move to workspace
            "alt-shift-F1"  = { type = "move-to-workspace"; name = "1"; };
            "alt-shift-F2"  = { type = "move-to-workspace"; name = "2"; };
            "alt-shift-F3"  = { type = "move-to-workspace"; name = "3"; };
            "alt-shift-F4"  = { type = "move-to-workspace"; name = "4"; };
            "alt-shift-F5"  = { type = "move-to-workspace"; name = "5"; };
            "alt-shift-F6"  = { type = "move-to-workspace"; name = "6"; };
            "alt-shift-F7"  = { type = "move-to-workspace"; name = "7"; };
            "alt-shift-F8"  = { type = "move-to-workspace"; name = "8"; };
            "alt-shift-F9"  = { type = "move-to-workspace"; name = "9"; };
            "alt-shift-F10" = { type = "move-to-workspace"; name = "10"; };
            "alt-shift-F11" = { type = "move-to-workspace"; name = "11"; };
            "alt-shift-F12" = { type = "move-to-workspace"; name = "12"; };
          };
        }
      '';
    };
  };

  config = mkIf cfg.enable {
    home.packages = [ cfg.package ];

    xdg.configFile = {
      "jay/config.toml" = mkIf (cfg.settings != { }) {
        source = tomlFormat.generate "config.toml" cfg.settings;
      };

      "jay/config.so" = mkIf (cfg.library != null) {
        source = "${cfg.library}/lib/config.so";
      };
    };
  };
}
