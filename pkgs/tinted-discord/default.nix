{
  lib,
  writeText,
}:
{
  themeFor =
    palette:
    let
      slots = lib.attrNames palette.withHashtag;
    in
    writeText "discord-theme-${palette.meta.slug or "tinted"}.css" (
      builtins.replaceStrings (map (s: "@${s}@") slots) (map (s: palette.withHashtag.${s}) slots) (
        builtins.readFile ./discord.css
      )
    );
}
