{ config, lib, ... }:
let
  inherit (config.theme) colors cursor fonts;
in
{
  # X11 clients under Xwayland read their font and colours from here rather
  # than from any of the Wayland-side configuration.
  files.".Xresources" = {
    generator = lib.generators.toKeyValueLines { separator = ": "; };

    value = with colors.withHashtag; {
      "*.faceName" = fonts.monospace.name;
      "*.faceSize" = fonts.sizes.terminal;
      "*.renderFont" = true;

      "*background" = background;
      "*foreground" = foreground;
      "*cursorColor" = foreground;

      "*color0" = base00;
      "*color1" = base08;
      "*color2" = base0B;
      "*color3" = base0A;
      "*color4" = base0D;
      "*color5" = base0E;
      "*color6" = base0C;
      "*color7" = base05;
      "*color8" = selection;
      "*color9" = base08;
      "*color10" = base0B;
      "*color11" = base0A;
      "*color12" = base0D;
      "*color13" = base0E;
      "*color14" = base0C;
      "*color15" = base07;
      "*color16" = base09;
      "*color17" = base0F;
      "*color18" = base01;
      "*color19" = base02;
      "*color20" = base04;
      "*color21" = base06;

      "Xcursor.theme" = cursor.name;
      "Xcursor.size" = cursor.size;
    };
  };

}
