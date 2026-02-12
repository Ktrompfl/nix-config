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
  ];

  # TODO: refine what really needs to be persisted here
  preservation.preserveAt.state-dir.directories = [
    ".local/cache"
    ".local/share"
    ".local/state"
  ];

  home.packages = [
    pkgs.xdg-utils # xdg-open
  ];
}
