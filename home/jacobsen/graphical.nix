{ pkgs, ... }:
{
  imports = [
    ./desktop
    ./services
    ./wayland

    ./programs/firefox
    # ./programs/vscode

    ./programs/chromium.nix
    ./programs/discord.nix
    ./programs/imv.nix
    ./programs/mpv.nix
    ./programs/satty.nix
    ./programs/seafile.nix
    ./programs/signal.nix
    ./programs/spotify.nix
    ./programs/thunderbird.nix
    ./programs/zathura.nix
    ./programs/zed.nix
    ./programs/zotero.nix
  ];

  packages = with pkgs; [
    better-control

    # audio tools
    alsa-scarlett-gui
    pwvucontrol # Pipewire Volume Control

    # multi media
    libreoffice
  ];
}
