{ pkgs, ... }:
{
  # font packages; setup is done by stylix
  home.packages = with pkgs; [
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
  ];
}
