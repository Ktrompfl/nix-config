{ config, lib, ... }:
{
  programs.zathura = {
    enable = true;
    options =
      let
        getColorCh = colorName: channel: config.lib.stylix.colors."${colorName}-rgb-${channel}";
        # rgb = color: ''rgb(${getColorCh color "r"}, ${getColorCh color "g"}, ${getColorCh color "b"})'';
        rgba =
          color: alpha:
          "rgba(${getColorCh color "r"}, ${getColorCh color "g"}, ${getColorCh color "b"}, ${builtins.toString alpha})";
      in
      {
        # stylix overrides
        # the pdf background is rendered on top of the (already transparent) window background
        # hence set the pdf background completely transparent to achieve matching results
        # still set the color to base00 since recolor-keephue is enable and takes this into account to transform colors
        recolor-lightcolor = lib.mkForce (rgba "base00" 0.0);

        # styling
        adjust-open = "width"; # startup option
        window-title-home-tilde = true;

        # recolor
        recolor = true;
        recolor-keephue = true; # keep original color
        recolor-reverse-video = true;

        # features
        selection-clipboard = "clipboard"; # Enable copy to clipboard
        synctex = true;
        sandbox = "none"; # Disable sandbox, hence allowing to follow urls
      };
  };

  xdg.mimeApps.defaultApplications = {
    "application/pdf" = "org.pwmt.zathura-pdf-mupdf.desktop";
  };
}
