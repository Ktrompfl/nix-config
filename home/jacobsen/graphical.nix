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
    ./programs/satty.nix
    ./programs/seafile.nix
    ./programs/signal.nix
    ./programs/spotify.nix
    ./programs/thunderbird.nix
    ./programs/viewnior.nix
    ./programs/vlc.nix
    ./programs/zathura.nix
    ./programs/zotero.nix
  ];

  packages = with pkgs; [
    baobab
    better-control
    nwg-look # for debug: wayland native gtk3 settings editor, like lxappearance

    # audio tools
    alsa-scarlett-gui
    pwvucontrol # Pipewire Volume Control

    # multi media
    libreoffice
    sqlitebrowser
  ];
}
