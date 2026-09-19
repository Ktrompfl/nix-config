{
  config,
  helpers,
  pkgs,
  ...
}:
let
  inherit (config.theme) colors fonts;
in
{
  apps.mpv = {
    package = pkgs.mpv;

    jail.permissions =
      c: with c; [
        viewer
        network # streaming urls
      ];

    files.".config/mpv/mpv.conf" = {
      mutable = false;
      generator = helpers.generators.toKeyValueLines { quote = true; };

      value = {
        osd-font = fonts.sansSerif.name;
        sub-font = fonts.sansSerif.name;
      }
      // (with colors.withHashtag; {
        background-color = base00;

        osd-back-color = base01;
        osd-border-color = base01;
        osd-color = base05;
        osd-shadow-color = base00;
      });
    };
  };
}
