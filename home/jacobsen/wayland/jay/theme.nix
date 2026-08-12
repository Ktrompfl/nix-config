{ config, ... }:
let
  font = config.stylix.fonts.monospace.name;
in
{
  # Colors, fonts, and sizes, all of them following the active stylix scheme.
  theme = with config.lib.stylix.colors.withHashtag; {
    inherit font;
    bg-color = base00;

    border-width = 1;
    border-color = base03;

    title-height = 16;
    title-font = font;
    attention-requested-bg-color = base09;
    captured-focused-title-bg-color = base08;
    captured-unfocused-title-bg-color = base0A;
    focused-inactive-title-bg-color = base03;
    focused-inactive-title-text-color = base05;
    focused-title-bg-color = base0D;
    focused-title-text-color = base00;
    separator-color = base03;
    unfocused-title-bg-color = base03;
    unfocused-title-text-color = base05;
    highlight-color = base0B;

    bar-position = "bottom";
    bar-height = 16;
    bar-separator-width = 1;
    bar-font = font;
    bar-bg-color = base01;
    bar-status-text-color = base05;
  };
}
