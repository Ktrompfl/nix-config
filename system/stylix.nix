{
  inputs,
  lib,
  pkgs,
  ...
}:
{
  imports = [ inputs.stylix.nixosModules.stylix ];

  stylix = {
    enable = lib.mkDefault true;

    # base16 tinted themes from https://github.com/tinted-theming/schemes
    # preview themes at https://tinted-theming.github.io/tinted-gallery/
    base16Scheme = "${pkgs.base16-schemes}/share/themes/rose-pine.yaml";
    polarity = "dark"; # either / light / dark

    cursor = {
      package = pkgs.rose-pine-cursor; # based on BreezeX
      name = "BreezeX-RosePine-Linux";
      size = 24;
    };

    icons = {
      enable = true;
      dark = "Papirus-Dark";
      light = "Papirus-Light";
      package = pkgs.papirus-icon-theme;
    };

    fonts = {
      serif = {
        package = pkgs.dejavu_fonts;
        name = "DejaVu Serif";
      };

      sansSerif = {
        package = pkgs.dejavu_fonts;
        name = "DejaVu Sans";
      };

      monospace = {
        package = pkgs.nerd-fonts.jetbrains-mono;
        name = "JetBrains Mono Nerd Font";
      };

      emoji = {
        package = pkgs.noto-fonts-color-emoji;
        name = "Noto Color Emoji";
      };
    };

    # opacity =
    #   let
    #     opacity = 0.8;
    #   in
    #   {
    #     applications = opacity;
    #     desktop = opacity;
    #     terminal = opacity;
    #     popups = opacity;
    #   };
  };

  # specialisation for light theme
  # specialisation.light = {
  #   inheritParentConfig = true;
  #   configuration = {
  #     stylix = {
  #       polarity = lib.mkForce "light";
  #       base16Scheme = lib.mkForce "${pkgs.base16-schemes}/share/themes/rose-pine-dawn.yaml";
  #       cursor.name = lib.mkForce "BreezeX-RosePineDawn-Linux";
  #     };
  #   };
  # };
}
