{
  config,
  lib,
  pkgs,
  ...
}:
{
  packages = [ pkgs.swaylock ];

  xdg.config.files."swaylock/config" = {
    generator = lib.generators.toKeyValueLines { flags = true; };

    value = with config.theme.colors.withoutHashtag; {
      color = base00;
      scaling = "fill";
      separator-color = "00000000";

      inside-color = base00;
      inside-clear-color = base00;
      inside-caps-lock-color = base00;
      inside-ver-color = base00;
      inside-wrong-color = base00;

      ring-color = base01;
      ring-clear-color = base08;
      ring-caps-lock-color = base01;
      ring-ver-color = base0B;
      ring-wrong-color = base08;

      key-hl-color = base0B;

      text-color = base05;
      text-clear-color = base05;
      text-caps-lock-color = base05;
      text-ver-color = base05;
      text-wrong-color = base05;

      layout-bg-color = base00;
      layout-border-color = base01;
      layout-text-color = base05;

      ignore-empty-password = true;
      line-uses-inside = true;
    };
  };
}
