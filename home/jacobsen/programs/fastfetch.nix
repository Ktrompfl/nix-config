{ pkgs, ... }:
{

  programs.fastfetch = {
    enable = true;
    settings = {
      modules = [
        "title"
        "separator"
        "os"
        "host"
        "kernel"
        "uptime"
        "shell"
        "display"
        "de"
        "wm"
        "wmtheme"
        "theme"
        "icons"
        "font"
        "cursor"
        "terminal"
        "terminalfont"
        "cpu"
        "gpu"
        "memory"
        "swap"
        "disk"
        "wifi"
        "localip"
        "publicip"
        "battery"
        "poweradapter"
        "break"
        "colors"
      ];
    };
  };

  home.packages = [
    pkgs.mesa-demos # required for graphic info with inxi (formerly glxinfo)
  ];
}
