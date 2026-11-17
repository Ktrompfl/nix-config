{
  config,
  helpers,
  ...
}:
let
  inherit (config.theme) cursor icons;
in
{
  packages = [
    icons.package
    cursor.package
  ];

  # a real copy, not a link into the store: Steam's client container shares
  # the home directory but not /nix/store, /etc/profiles or /run/current-system
  systemd.tmpfiles.rules = [
    "C %h/.icons/${cursor.name} - - - - ${cursor.package}/share/icons/${cursor.name}"
  ];

  xdg.data.files."icons/default/index.theme" = {
    generator = helpers.generators.toINI { };

    value."Icon Theme" = {
      Name = "Default";
      Comment = "Default Cursor Theme";
      Inherits = cursor.name;
    };
  };

  environment.sessionVariables = {
    XCURSOR_THEME = cursor.name;
    XCURSOR_SIZE = toString cursor.size;
  };
}
