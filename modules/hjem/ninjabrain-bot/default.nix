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
        mask: modifier: lib.bitOr mask modifiers.${modifier}
      ) 0 hotkey.modifiers;
    }
  ) hotkeys;

  settings = lib.filterAttrs (_: value: value != null) (
    lib.removeAttrs cfg.settings [ "_module" ] // hotkeyEntries
  );

  palette = lib.genAttrs (map (digit: "base0${digit}") (lib.stringToCharacters "0123456789ABCDEF")) (
    name: config.theme.colors.withoutHashtag.${name}
  );

  json = (pkgs.formats.json { }).generate;

  prefsArgs = [
    "--settings"
    "${json "ninjabrain-bot-settings.json" settings}"
  ]
  ++ lib.optionals cfg.theme [
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
  options.programs.ninjabrain-bot = import ./options.nix {
    inherit
      actions
      cfg
      defaultKeys
      hotkeyType
      lib
      mkEnumOption
      pkgs
      ;
  };

  config = lib.mkIf cfg.enable {
    files.".java/.userPrefs/ninjabrainbot/prefs.xml" = {
      type = "copy";
      clobber = true;
      source = prefs;
    };

    packages = [ cfg.package ];
  };
}
