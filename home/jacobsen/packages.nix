{ pkgs, ... }:
{
  packages = with pkgs; [
    # nix tools
    manix

    # graphical tools
    baobab
    better-control
    gpu-screen-recorder-gtk
    nwg-look # for debug: wayland native gtk3 settings editor, like lxappearance

    # audio tools
    alsa-scarlett-gui
    pwvucontrol # Pipewire Volume Control

    # multi media
    libreoffice
    sqlitebrowser

    # languages
    php
    typst
  ];
}
