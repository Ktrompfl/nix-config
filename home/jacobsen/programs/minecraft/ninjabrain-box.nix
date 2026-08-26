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
        # green boat
        mc-version = "pre-1.19";
        sensitivity = 0.02291165;
        sigma-boat = 0.0007;
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
        start-hidden = false;
        only-when-focused = [ "waywall" ]; # `show` overrides it until refocus
        start-with-throws = false;
        mode = "unbound"; # restricted
      };

      palette = lib.genAttrs (map (digit: "base0${digit}") (lib.stringToCharacters "0123456789ABCDEF")) (
        name: config.theme.colors.withHashtag.${name}
      );
    };
  };
}
