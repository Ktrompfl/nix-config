{ lib }:
let
  hexDigits = {
    "0" = 0;
    "1" = 1;
    "2" = 2;
    "3" = 3;
    "4" = 4;
    "5" = 5;
    "6" = 6;
    "7" = 7;
    "8" = 8;
    "9" = 9;
    "a" = 10;
    "b" = 11;
    "c" = 12;
    "d" = 13;
    "e" = 14;
    "f" = 15;
  };

  # "1f" -> 31
  hexPairToInt =
    pair:
    let
      chars = lib.stringToCharacters (lib.toLower pair);
    in
    16 * hexDigits.${builtins.elemAt chars 0} + hexDigits.${builtins.elemAt chars 1};

  slotPattern = ''^[[:space:]]*(base[0-1][0-9A-Fa-f])[[:space:]]*:[[:space:]]*"?#?([0-9a-fA-F]{6})"?[[:space:]]*$'';

  metaPattern = ''^[[:space:]]*(system|name|author|slug|variant)[[:space:]]*:[[:space:]]*"?(.*[^"[:space:]])"?[[:space:]]*$'';

  aliases = {
    background = "base00";
    surface = "base01";
    selection = "base02";
    muted = "base03";
    subtle = "base04";
    foreground = "base05";
    foregroundBright = "base06";
    surfaceBright = "base07";

    red = "base08";
    orange = "base09";
    yellow = "base0A";
    green = "base0B";
    cyan = "base0C";
    blue = "base0D";
    magenta = "base0E";
    brown = "base0F";

    error = "base08";
    warning = "base09";
    highlight = "base0A";
    success = "base0B";
    info = "base0C";
    accent = "base0D";
    keyword = "base0E";
    deprecated = "base0F";
  };
in
schemeFile:
let
  matches = lib.filter (m: m != null) (
    map (builtins.match slotPattern) (lib.splitString "\n" (builtins.readFile schemeFile))
  );

  # Slot names are upper-cased (base0A, not base0a) to match tinted-theming.
  slots = lib.listToAttrs (
    map (m: {
      name =
        lib.substring 0 5 (builtins.elemAt m 0) + lib.toUpper (lib.substring 5 1 (builtins.elemAt m 0));
      value = lib.toLower (builtins.elemAt m 1);
    }) matches
  );

  meta = lib.listToAttrs (
    map (m: lib.nameValuePair (builtins.elemAt m 0) (builtins.elemAt m 1)) (
      lib.filter (m: m != null) (
        map (builtins.match metaPattern) (lib.splitString "\n" (builtins.readFile schemeFile))
      )
    )
  );

  # Slots plus aliases, so every accessor takes either spelling.
  named = slots // lib.mapAttrs (_: slot: slots.${slot}) aliases;

  resolve =
    name:
    named.${name}
      or (throw "unknown colour '${name}'; expected a base16 slot or one of: ${lib.concatStringsSep ", " (lib.attrNames aliases)}");

  channels =
    name:
    let
      hex = resolve name;
    in
    {
      r = hexPairToInt (lib.substring 0 2 hex);
      g = hexPairToInt (lib.substring 2 2 hex);
      b = hexPairToInt (lib.substring 4 2 hex);
    };
in
assert lib.assertMsg
  (lib.elem (lib.length (lib.attrNames slots)) [
    16
    24
  ])
  "scheme ${toString schemeFile} defined ${toString (lib.length (lib.attrNames slots))} slots, expected 16 or 24";
{
  # The scheme's own metadata: system, name, author, slug, variant.
  inherit meta;

  # The raw slots, without the aliases, for templates that iterate them.
  inherit slots;

  # Attribute sets, for `with` blocks: `with colors.withHashtag; "${error}"`.
  withHashtag = lib.mapAttrs (_: v: "#${v}") named;
  withoutHashtag = named;

  # Functions, for the formats that need something other than plain hex.
  inherit channels;

  # "#eb6f92"
  hex = name: "#${resolve name}";

  # "eb6f92ff" — RRGGBBAA, as fuzzel and satty want. `alpha` is 0..1 and is
  # rendered as the two hex digits those formats expect.
  rgbaHex =
    name: alpha:
    let
      byte = builtins.floor (alpha * 255 + 0.5);
      digits = "0123456789abcdef";
      nibble = n: lib.substring n 1 digits;
    in
    "${resolve name}${nibble (byte / 16)}${nibble (lib.mod byte 16)}";

  # "eb6f92ff" — the opaque case, which is almost all of them.
  opaque = name: "${resolve name}ff";

  # "rgb(235, 111, 146)"
  rgb =
    name:
    let
      c = channels name;
    in
    "rgb(${toString c.r}, ${toString c.g}, ${toString c.b})";

  # "rgba(235, 111, 146, 0.5)"
  rgba =
    name: alpha:
    let
      c = channels name;
    in
    "rgba(${toString c.r}, ${toString c.g}, ${toString c.b}, ${toString alpha})";
}
