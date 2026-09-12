{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.theme) colors fonts;
in
{
  packages = [ pkgs.mpv ];

  xdg.config.files."mpv/mpv.conf" = {
    generator = lib.generators.toKeyValueLines { quote = true; };

    value = {
      osd-font = fonts.sansSerif.name;
      sub-font = fonts.sansSerif.name;
    }
    // (with colors.withHashtag; {
      background-color = "#000000";

      osd-back-color = base01;
      osd-border-color = base01;
      osd-color = base05;
      osd-shadow-color = base00;
    });
  };
}
