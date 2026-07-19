{
  config,
  inputs,
  lib,
  pkgs,
  ...
}:
{
  imports = [ inputs.jay.homeManagerModules.default ];

  wayland.windowManager.jay = {
    enable = true;
    library = pkgs.jay-config-lib;
  };

  # Theme/styling values only, read by jay-config at reload time (see
  # jay-config/src/theme.rs) instead of being compiled into config.so, so
  # switching stylix schemes doesn't require rebuilding the config crate.
  xdg.configFile."jay/theme.toml".text = lib.concatStrings (
    lib.mapAttrsToList (name: value: "${name} = ${builtins.toJSON (toString value)}\n") {
      cursor_theme = config.home.pointerCursor.name;
      cursor_size = config.home.pointerCursor.size;
      gtk_theme = config.gtk.theme.name;
      qt_platform_theme = config.qt.platformTheme.name;
      qt_style = config.qt.style.name;
      monospace_font = config.stylix.fonts.monospace.name;

      base00 = config.lib.stylix.colors.base00;
      base01 = config.lib.stylix.colors.base01;
      base02 = config.lib.stylix.colors.base02;
      base03 = config.lib.stylix.colors.base03;
      base04 = config.lib.stylix.colors.base04;
      base05 = config.lib.stylix.colors.base05;
      base06 = config.lib.stylix.colors.base06;
      base07 = config.lib.stylix.colors.base07;
      base08 = config.lib.stylix.colors.base08;
      base09 = config.lib.stylix.colors.base09;
      base0a = config.lib.stylix.colors.base0A;
      base0b = config.lib.stylix.colors.base0B;
      base0c = config.lib.stylix.colors.base0C;
      base0d = config.lib.stylix.colors.base0D;
      base0e = config.lib.stylix.colors.base0E;
      base0f = config.lib.stylix.colors.base0F;
    }
  );

  # extra packages used in the jay config
  home.packages = with pkgs; [
    playerctl
    wl-mirror
  ];

  # persist logs and session management
  preservation.preserveAt.state-dir.directories = [ ".local/share/jay" ];
}
