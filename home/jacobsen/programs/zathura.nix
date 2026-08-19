{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.theme.colors) rgb rgba;
in
{
  packages = [ pkgs.zathura ];

  xdg.config.files."zathura/zathurarc" = {
    generator = lib.generators.toKeyValueLines {
      mkKey = key: "set ${key}";
      separator = "\t";
      quote = true;
    };

    value = {
      default-bg = rgba "background" 1.0;
      default-fg = rgb "foreground";

      statusbar-bg = rgb "surface";
      statusbar-fg = rgb "foreground";
      inputbar-bg = rgb "background";
      inputbar-fg = rgb "foreground";

      completion-bg = rgb "selection";
      completion-fg = rgb "foreground";
      completion-group-bg = rgb "surface";
      completion-group-fg = rgb "foreground";
      completion-highlight-bg = rgb "accent";
      completion-highlight-fg = rgb "background";

      index-bg = rgb "background";
      index-fg = rgb "foreground";
      index-active-bg = rgb "selection";
      index-active-fg = rgb "foreground";

      notification-bg = rgb "background";
      notification-fg = rgb "foreground";
      notification-error-bg = rgb "background";
      notification-error-fg = rgb "error";
      notification-warning-bg = rgb "background";
      notification-warning-fg = rgb "warning";

      render-loading-bg = rgb "background";
      render-loading-fg = rgb "foreground";

      highlight-color = rgba "subtle" 0.3;
      highlight-active-color = rgba "accent" 0.3;
      highlight-fg = rgb "foreground";

      signature-error-color = rgb "error";
      signature-success-color = rgb "success";
      signature-warning-color = rgb "warning";

      recolor-darkcolor = rgb "foreground";

      # the pdf background is rendered on top of the (already transparent)
      # window background, so it is made fully transparent to match. The
      # colour still matters because recolor-keephue transforms against it.
      recolor-lightcolor = rgba "background" 0.0;

      # styling
      adjust-open = "width";
      window-title-home-tilde = "true";

      # recolor
      recolor = "true";
      recolor-keephue = "true";
      recolor-reverse-video = "true";

      # features
      selection-clipboard = "clipboard";
      synctex = "true";
    };
  };

  preservation.preserveAt.state-dir.directories = [
    ".local/share/zathura"
  ];
}
