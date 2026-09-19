{ config, pkgs, ... }:
let
  inherit (config.theme) fonts;
in
{
  # the defaults and fontconfig are set in system/theme
  packages = [
    fonts.serif.package
    fonts.sansSerif.package
    fonts.monospace.package
    fonts.emoji.package
  ]
  ++ (with pkgs; [
    geist-font
    nerd-fonts.fira-code
    nerd-fonts.geist-mono
    nerd-fonts.jetbrains-mono
    nerd-fonts.zed-mono
    redhat-official-fonts
    noto-fonts
    noto-fonts-cjk-sans
    noto-fonts-color-emoji
    font-awesome
    liberation_ttf
    roboto
  ]);
}
