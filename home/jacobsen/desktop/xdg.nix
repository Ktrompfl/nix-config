{ lib, pkgs, ... }:
let
  directories = {
    XDG_DESKTOP_DIR = "/home/jacobsen/Desktop";
    XDG_DOCUMENTS_DIR = "/home/jacobsen/Documents";
    XDG_DOWNLOAD_DIR = "/home/jacobsen/Downloads";
    XDG_MUSIC_DIR = "/home/jacobsen/Music";
    XDG_PICTURES_DIR = "/home/jacobsen/Pictures";
    XDG_PROJECTS_DIR = "/home/jacobsen/Projects";
    XDG_PUBLICSHARE_DIR = "/home/jacobsen/Public";
    XDG_SCREENSHOTS_DIR = "/home/jacobsen/Pictures/screenshots";
    XDG_TEMPLATES_DIR = "/home/jacobsen/Templates";
    XDG_VIDEOS_DIR = "/home/jacobsen/Videos";
  };
in
{
  packages = [ pkgs.xdg-utils ]; # xdg-open

  xdg.config.files = {
    # xdg-user-dirs reads the quoted form; the same paths are exported below
    # so that programs which only look at the environment agree with it.
    "user-dirs.dirs" = {
      generator = lib.generators.toKeyValueLines { quote = true; };
      value = directories;
    };

    "user-dirs.conf".text = "enabled=False";
  };

  environment.sessionVariables = directories // {
    XDG_CACHE_HOME = "/home/jacobsen/.local/cache";
  };

  preservation.preserveAt = {
    data-dir.directories = [
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

    state-dir.directories = [
      # xdg cache home
      ".local/cache"

      # xdg state home
      ".local/state"
    ];
  };
}
