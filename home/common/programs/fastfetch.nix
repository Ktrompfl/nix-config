{ pkgs, ... }:
{
  packages = with pkgs; [
    fastfetch
    mesa-demos # required for graphic info with inxi (formerly glxinfo)
  ];

  xdg.config.files."fastfetch/config.jsonc" = {
    generator = (pkgs.formats.json { }).generate "fastfetch-config.jsonc";
    value.modules = [
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
}
