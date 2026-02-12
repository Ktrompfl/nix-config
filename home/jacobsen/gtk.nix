{ config, pkgs, ... }:
{
  home.packages = with pkgs; [ glib ];

  stylix.targets.gtk = {
    enable = true;
    flatpakSupport.enable = false;
  };

  # GTK themeing
  gtk.gtk3.bookmarks = [
    "file://${config.home.homeDirectory}/Archive"
    "file://${config.home.homeDirectory}/Documents"
    "file://${config.home.homeDirectory}/Downloads"
    "file://${config.home.homeDirectory}/Music"
    "file://${config.home.homeDirectory}/Pictures"
    "file://${config.home.homeDirectory}/Repositories"
    "file://${config.home.homeDirectory}/Videos"
  ];
}
