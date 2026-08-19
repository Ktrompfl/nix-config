{
  config,
  pkgs,
  ...
}:
{
  packages = with pkgs; [
    satty
    grim
    slurp
    wl-clipboard
  ];

  xdg.config.files."satty/config.toml" = {
    generator = (pkgs.formats.toml { }).generate "satty-config.toml";
    value = {
      general = {
        fullscreen = true;
        copy-command = "wl-copy";
        output-filename = "~/Pictures/screenshots/%Y-%m-%d-%H%M%S.png";
        initial-tool = "crop";
        actions-on-enter = [
          "save-to-clipboard"
          "save-to-file"
          "exit"
        ];
        actions-on-escape = [ "exit" ];
      };

      font = {
        family = config.theme.fonts.sansSerif.name;
        style = "Regular";
      };

      color-palette.palette = with config.theme.colors; [
        "#${opaque "error"}" # errors/highlights
        "#${opaque "success"}" # success/ok
        "#${opaque "highlight"}" # warnings
        "#${opaque "accent"}" # info/accent
        "#${opaque "keyword"}" # extra accent
      ];
    };
  };
}
