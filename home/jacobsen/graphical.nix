{ pkgs, ... }:
{
  imports = [
    ./desktop
    ./services
    ./wayland

    ./programs/firefox
    ./programs/zed
    # ./programs/vscode

    ./programs/chromium.nix
    ./programs/discord.nix
    ./programs/mpv.nix
    ./programs/satty.nix
    ./programs/seafile.nix
    ./programs/signal.nix
    ./programs/spotify.nix
    ./programs/thunderbird.nix
    ./programs/zathura.nix
    ./programs/zotero.nix
  ];

  packages = with pkgs; [
    better-control

    # audio tools
    alsa-scarlett-gui
    pwvucontrol # Pipewire Volume Control

    # multi media
    imv
    libreoffice
  ];
}
