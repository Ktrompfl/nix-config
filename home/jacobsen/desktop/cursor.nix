{ config, lib, ... }:
let
  inherit (config.theme) cursor icons;
in
{
  packages = [
    icons.package
    cursor.package
  ];

  # The cursor has to be visible to X11 clients under Xwayland too, which read
  # it from ~/.icons rather than from the theme packages.
  files.".icons/default/index.theme" = {
    generator = lib.generators.toINI { };

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
