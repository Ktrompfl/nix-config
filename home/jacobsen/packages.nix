{ pkgs, ... }:
{
  home.packages = with pkgs; [
    # wayland tools
    grim
    slurp
    wev
    wl-clipboard
    wl-mirror

    # graphical tools
    baobab
    better-control
    nwg-look # for debug: wayland native gtk3 settings editor, like lxappearance

    # audio tools
    # coppwr # Low level control GUI for the PipeWire multimedia server
    # easyeffects # Audio effects for PipeWire applications
    # helvum # GTK patchbay for pipewire
    # pavucontrol # PulseAudio Volume Control
    pwvucontrol # Pipewire Volume Control
    # sonusmix # Next-gen Pipewire audio routing tool

    # multi media
    inkscape
    libreoffice

    # languages
    ghc # haskell
    php
    typst
  ];
}
