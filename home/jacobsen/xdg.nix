{ pkgs, config, ... }:
{
  home.preferXdgDirectories = true;

  xdg = {
    # setup xdg home directories
    enable = true;
    cacheHome = config.home.homeDirectory + "/.local/cache";

    userDirs = {
      enable = true;
      createDirectories = true;
      extraConfig = {
        XDG_SCREENSHOTS_DIR = "${config.xdg.userDirs.pictures}/screenshots";
      };
    };
  };

  # enable xdg mime types
  xdg.mimeApps.enable = true;

  # persist xdg home directories
  preservation.preserveAt.data-dir.directories = [
    # xdg home directories
    "Archive"
    "Desktop"
    "Documents"
    "Downloads"
    "Games"
    "Music"
    "Pictures"
    "Programs"
    "Public"
    "Repositories"
    "Templates"
    "Videos"

    # xdg data home, preservation delegated to individual programs
    # ".local/share"
  ];

  preservation.preserveAt.state-dir.directories = [
    # xdg cache home
    ".local/cache"

    # xdg state home
    ".local/state"
  ];

  home.packages = [
    pkgs.xdg-utils # xdg-open
  ];
}
