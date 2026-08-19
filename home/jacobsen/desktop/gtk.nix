{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.theme)
    colors
    fonts
    icons
    cursor
    ;
  settings = {
    gtk-theme-name = "adw-gtk3";
    gtk-icon-theme-name = icons.name;
    gtk-cursor-theme-name = cursor.name;
    gtk-cursor-theme-size = cursor.size;
    gtk-font-name = "${fonts.sansSerif.name} ${toString fonts.sizes.applications}";
  };

  namedColors = with colors.withHashtag; ''
    @define-color accent_color ${base0D};
    @define-color accent_bg_color ${base0D};
    @define-color accent_fg_color ${base00};
    @define-color destructive_color ${base08};
    @define-color destructive_bg_color ${base08};
    @define-color destructive_fg_color ${base00};
    @define-color success_color ${base0B};
    @define-color success_bg_color ${base0B};
    @define-color success_fg_color ${base00};
    @define-color warning_color ${base09};
    @define-color warning_bg_color ${base09};
    @define-color warning_fg_color ${base00};
    @define-color error_color ${base08};
    @define-color error_bg_color ${base08};
    @define-color error_fg_color ${base00};
    @define-color window_bg_color ${base00};
    @define-color window_fg_color ${base05};
    @define-color view_bg_color ${base00};
    @define-color view_fg_color ${base05};
    @define-color headerbar_bg_color ${base01};
    @define-color headerbar_fg_color ${base05};
    @define-color headerbar_border_color ${base01};
    @define-color headerbar_backdrop_color @window_bg_color;
    @define-color headerbar_shade_color rgba(0, 0, 0, 0.07);
    @define-color headerbar_darker_shade_color rgba(0, 0, 0, 0.07);
    @define-color sidebar_bg_color ${base01};
    @define-color sidebar_fg_color ${base05};
    @define-color sidebar_backdrop_color @window_bg_color;
    @define-color sidebar_shade_color rgba(0, 0, 0, 0.07);
    @define-color secondary_sidebar_bg_color @sidebar_bg_color;
    @define-color secondary_sidebar_fg_color @sidebar_fg_color;
    @define-color secondary_sidebar_backdrop_color @window_bg_color;
    @define-color secondary_sidebar_shade_color rgba(0, 0, 0, 0.07);
    @define-color card_bg_color ${base01};
    @define-color card_fg_color ${base05};
    @define-color card_shade_color rgba(0, 0, 0, 0.07);
    @define-color dialog_bg_color ${base01};
    @define-color dialog_fg_color ${base05};
    @define-color popover_bg_color ${base01};
    @define-color popover_fg_color ${base05};
    @define-color popover_shade_color rgba(0, 0, 0, 0.07);
    @define-color thumbnail_bg_color ${base01};
    @define-color thumbnail_fg_color ${base05};
    @define-color shade_color rgba(0, 0, 0, 0.07);
    @define-color scrollbar_outline_color ${base02};
  '';
in
{
  packages = [
    pkgs.glib
    pkgs.adw-gtk3
    icons.package
    cursor.package
  ];

  files.".gtkrc-2.0" = {
    generator = lib.generators.toGtk2;
    value = settings;
  };

  xdg.config.files = {
    "gtk-3.0/settings.ini" = {
      generator = lib.generators.toGtkINI;
      value = settings;
    };

    "gtk-4.0/settings.ini" = {
      generator = lib.generators.toGtkINI;
      value = settings;
    };

    "gtk-3.0/gtk.css".text = namedColors;
    "gtk-4.0/gtk.css".text = namedColors;

    "gtk-3.0/bookmarks".text = lib.concatStringsSep "\n" [
      "file:///home/jacobsen/Archive"
      "file:///home/jacobsen/Documents"
      "file:///home/jacobsen/Downloads"
      "file:///home/jacobsen/Music"
      "file:///home/jacobsen/Pictures"
      "file:///home/jacobsen/Repositories"
      "file:///home/jacobsen/Videos"
    ];
  };

  # GTK2 has no XDG location of its own, so applications are pointed at the
  # file explicitly.
  environment.sessionVariables.GTK2_RC_FILES = "/home/jacobsen/.gtkrc-2.0";
}
