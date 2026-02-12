{ pkgs, ... }:
{
  home.packages = with pkgs; [
    qt5.qtwayland
    libsForQt5.qtstyleplugin-kvantum
    libsForQt5.qt5ct
    qt6Packages.qtstyleplugin-kvantum
    qt6Packages.qt6ct
  ];
}
