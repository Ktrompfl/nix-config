{
  config,
  lib,
  pkgs,
  ...
}:
let
  inherit (config.theme) fonts icons;
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
      value.General.theme = "rose-pine-iris";
    };

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
    QT_QPA_PLATFORMTHEME = "qtct";
    QT_STYLE_OVERRIDE = "kvantum";
  };
}
