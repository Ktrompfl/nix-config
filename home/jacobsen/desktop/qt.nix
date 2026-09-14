{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.theme) fonts icons;

  theme = "rose-pine-iris";
  themePath = "${pkgs.rose-pine-kvantum}/share/Kvantum/themes/${theme}";

  qt5Plugins = map (package: "${package}/lib/qt-${pkgs.libsForQt5.qtbase.version}/plugins") [
    pkgs.libsForQt5.qt5ct
    pkgs.libsForQt5.qtstyleplugin-kvantum
  ];

  qt6Plugins = map (package: "${package}/lib/qt-6/plugins") [
    pkgs.qt6Packages.qt6ct
    pkgs.qt6Packages.qtstyleplugin-kvantum
  ];

  qtct = {
    Appearance = {
      custom_palette = false;
      icon_theme = icons.name;
      standard_dialogs = "default";
      style = "kvantum";
    };

    Fonts = {
      fixed = ''"${fonts.monospace.name},${toString fonts.sizes.applications}"'';
      general = ''"${fonts.sansSerif.name},${toString fonts.sizes.applications}"'';
    };
  };
in
{
  packages = with pkgs; [
    rose-pine-kvantum

    qt5.qtwayland
    libsForQt5.qtstyleplugin-kvantum
    libsForQt5.qt5ct
    qt6Packages.qtstyleplugin-kvantum
    qt6Packages.qt6ct
  ];

  xdg.config.files = {
    "Kvantum/kvantum.kvconfig" = {
      generator = lib.generators.toINI { };
      value.General.theme = theme;
    };

    "Kvantum/${theme}/${theme}.kvconfig".source = "${themePath}/${theme}.kvconfig";
    "Kvantum/${theme}/${theme}.svg".source = "${themePath}/${theme}.svg";

    "qt5ct/qt5ct.conf" = {
      generator = lib.generators.toINI { };
      value = qtct;
    };

    "qt6ct/qt6ct.conf" = {
      generator = lib.generators.toINI { };
      value = qtct;
    };
  };

  environment.sessionVariables = {
    QT_PLUGIN_PATH = lib.concatStringsSep ":" (qt5Plugins ++ qt6Plugins);
    QT_QPA_PLATFORM = "wayland;xcb";
    QT_QPA_PLATFORMTHEME = "qtct";
    QT_STYLE_OVERRIDE = "kvantum";
  };
}
