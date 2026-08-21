{
  config,
  lib,
  pkgs,
  ...
}:
{
  packages = [ pkgs.ninjabrain-box ];

  xdg.config.files."ninjabrain-box/config.toml" = {
    generator = (pkgs.formats.toml { }).generate "ninjabrain-box.toml";
    value = {
      bot.settings = {
        mc-version = "pre-1.19";
        angle-adjustment = "tall";
        boat-type = "green";
        use-precise-angle = true;
        sensitivity = 0.02291165;
        sigma-boat = 0.001;
        boat-error = 0.03;
      };

      window = {
        output = "DP-2";
        anchor = [
          "top"
          "right"
        ];
        coordinates = "chunk";
        predictions = 4;
        font = "${config.theme.fonts.monospace.package}/share/fonts/truetype/NerdFonts/JetBrainsMono/JetBrainsMonoNerdFontMono-Regular.ttf";
        font-size = 17.0;
        opacity = 0.85;
      };

      behavior = {
        # The predictions are up whenever the game is, and no longer: the
        # focus rule is the whole of it, so nothing has to remember to hide.
        start-hidden = false;
        only-when-focused = [ "waywall" ];

        # The throws are only worth the space while measuring, so waywall
        # turns them on with the tall macro and off again on the way out.
        start-with-throws = false;
      };

      palette = lib.genAttrs (map (digit: "base0${digit}") (lib.stringToCharacters "0123456789ABCDEF")) (
        name: config.theme.colors.withHashtag.${name}
      );
    };
  };
}
