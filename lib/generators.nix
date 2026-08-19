{ lib }:
let
  inherit (lib)
    concatLines
    concatStringsSep
    filterAttrs
    isBool
    isString
    mapAttrsToList
    optional
    ;

  mkValue =
    { quote }:
    value:
    let
      rendered = if isBool value then lib.boolToString value else toString value;
    in
    if quote then ''"${rendered}"'' else rendered;
in
{
  # One `key<separator>value` line per attribute, in attribute-name order.
  # Named apart from the nixpkgs `toKeyValue` it sits beside, which takes a
  # `mkKeyValue` function instead of these options.
  #
  #   quote      wrap values in double quotes
  #   flags      spell `true` as a bare key and drop `false` entirely, the way
  #              MangoHud and other flag-style formats write booleans
  #   mkKey      rewrite the key for formats that decorate it, such as
  #              zathura's `set x` or btop's `theme[x]`
  toKeyValueLines =
    {
      separator ? "=",
      quote ? false,
      flags ? false,
      mkKey ? lib.id,
    }:
    attrs:
    concatLines (
      mapAttrsToList (
        name: value:
        if flags && isBool value then
          mkKey name
        else
          "${mkKey name}${separator}${mkValue { inherit quote; } value}"
      ) (filterAttrs (_: value: !(flags && value == false)) attrs)
    );

  # ~/.ssh/config. ssh keeps the first value it sees for an option, so the
  # catch-all `Host *` block is emitted last rather than in attribute order.
  toSSHConfig =
    hosts:
    let
      # ssh spells booleans yes/no; true/false is rejected.
      mkSSHValue = value: if isBool value then (if value then "yes" else "no") else toString value;

      renderHost =
        name: options:
        concatLines (
          [ "Host ${name}" ] ++ mapAttrsToList (key: value: "  ${key} ${mkSSHValue value}") options
        );
    in
    concatStringsSep "\n" (
      mapAttrsToList renderHost (filterAttrs (name: _: name != "*") hosts)
      ++ optional (hosts ? "*") (renderHost "*" hosts."*")
    );

  # Mozilla's user.js, as used by both Firefox and Thunderbird. Every
  # preference is a `user_pref()` call, and anything that is not a scalar is
  # stored as a JSON string rather than as JSON.
  toMozillaPrefs =
    prefs:
    concatLines (
      mapAttrsToList (
        name: value:
        let
          encoded = if lib.isAttrs value || lib.isList value then builtins.toJSON value else value;
        in
        "user_pref(${builtins.toJSON name}, ${builtins.toJSON encoded});"
      ) prefs
    );

  # The profiles.ini both of them write for a single, default profile.
  toMozillaProfiles =
    { name }:
    lib.generators.toINI { } {
      General = {
        StartWithLastProfile = 1;
        Version = 2;
      };

      Profile0 = {
        Default = 1;
        IsRelative = 1;
        Name = name;
        Path = name;
      };
    };

  # ~/.gtkrc-2.0, where string values are quoted but numbers, booleans and the
  # GTK_* enum constants are bare.
  toGtk2 =
    settings:
    concatLines (
      mapAttrsToList (
        name: value:
        let
          quoted = isString value && !(lib.hasPrefix "GTK_" value);
        in
        "${name}=${mkValue { quote = quoted; } value}"
      ) settings
    );

  # gtk-3.0/settings.ini and gtk-4.0/settings.ini, which quote nothing.
  toGtkINI = settings: lib.generators.toINI { } { Settings = settings; };
}
