{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.theme;
  mkScheme = import ./palette.nix { inherit lib; };
in
{
  options.theme = {
    scheme = lib.mkOption {
      type = lib.types.path;
      default = "${pkgs.base16-schemes}/share/themes/rose-pine.yaml";
      description = ''
        A tinted-theming scheme file. Browse the gallery at
        <https://tinted-theming.github.io/tinted-gallery/>; every scheme in
        `pkgs.base16-schemes` is a valid value.
      '';
    };

    colors = lib.mkOption {
      type = lib.types.attrs;
      readOnly = true;
      description = ''
        The parsed palette, in three shapes: `withHashtag` (`"#191724"`),
        `withoutHashtag` (`"191724"`) and `rgb` (`{ r = 25; g = 23; b = 36; }`).
      '';
    };

    fonts = {
      serif = lib.mkOption {
        type = lib.types.attrs;
        default = {
          package = pkgs.dejavu_fonts;
          name = "DejaVu Serif";
        };
      };
      sansSerif = lib.mkOption {
        type = lib.types.attrs;
        default = {
          package = pkgs.dejavu_fonts;
          name = "DejaVu Sans";
        };
      };
      monospace = lib.mkOption {
        type = lib.types.attrs;
        default = {
          package = pkgs.nerd-fonts.jetbrains-mono;
          name = "JetBrains Mono Nerd Font";
        };
      };
      emoji = lib.mkOption {
        type = lib.types.attrs;
        default = {
          package = pkgs.noto-fonts-color-emoji;
          name = "Noto Color Emoji";
        };
      };

      sizes = {
        applications = lib.mkOption {
          type = lib.types.int;
          default = 12;
        };
        desktop = lib.mkOption {
          type = lib.types.int;
          default = 10;
        };
        popups = lib.mkOption {
          type = lib.types.int;
          default = 10;
        };
        terminal = lib.mkOption {
          type = lib.types.int;
          default = 12;
        };
      };
    };

    cursor = {
      package = lib.mkOption {
        type = lib.types.package;
        default = pkgs.rose-pine-cursor;
      };
      name = lib.mkOption {
        type = lib.types.str;
        default = "BreezeX-RosePine-Linux";
      };
      size = lib.mkOption {
        type = lib.types.int;
        default = 24;
      };
    };

    icons = {
      package = lib.mkOption {
        type = lib.types.package;
        default = pkgs.papirus-icon-theme;
      };
      name = lib.mkOption {
        type = lib.types.str;
        default = "Papirus-Dark";
      };
    };
  };

  config = {
    theme.colors = mkScheme cfg.scheme;

    fonts = {
      enableDefaultPackages = true;

      packages = [
        cfg.fonts.serif.package
        cfg.fonts.sansSerif.package
        cfg.fonts.monospace.package
        cfg.fonts.emoji.package
      ];

      fontconfig.defaultFonts = {
        serif = [ cfg.fonts.serif.name ];
        sansSerif = [ cfg.fonts.sansSerif.name ];
        monospace = [ cfg.fonts.monospace.name ];
        emoji = [ cfg.fonts.emoji.name ];
      };
    };
  };
}
