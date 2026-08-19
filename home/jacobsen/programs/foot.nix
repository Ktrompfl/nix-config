{
  config,
  lib,
  pkgs,
  ...
}:
{
  packages = [ pkgs.foot ];

  xdg.config.files."foot/foot.ini" = {
    generator = lib.generators.toINI { };
    value = {
      main = {
        # the server runs as a systemd unit, whose PATH is minimal
        shell = lib.getExe pkgs.fish;
        term = "xterm-256color";
        font = "${config.theme.fonts.monospace.name}:size=${toString config.theme.fonts.sizes.terminal}";
        dpi-aware = "no";
        initial-color-theme = "dark";
      };

      colors-dark = with config.theme.colors.withoutHashtag; {
        alpha = "1.000000";
        background = base00;
        foreground = base05;
        regular0 = base00;
        regular1 = base08;
        regular2 = base0B;
        regular3 = base0A;
        regular4 = base0D;
        regular5 = base0E;
        regular6 = base0C;
        regular7 = base05;
        bright0 = base03;
        bright1 = base08;
        bright2 = base0B;
        bright3 = base0A;
        bright4 = base0D;
        bright5 = base0E;
        bright6 = base0C;
        bright7 = base07;
        "16" = base09;
        "17" = base0F;
        "18" = base01;
        "19" = base02;
        "20" = base04;
        "21" = base06;
      };

      bell.system = "no";
      cursor = {
        style = "beam";
        blink = "true";
      };
      mouse.hide-when-typing = "yes";
    };
  };

  environment.sessionVariables.TERMINAL = "footclient";
}
