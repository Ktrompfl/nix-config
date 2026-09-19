{
  config,
  generators,
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

  xdg.data.files."icons/default/index.theme" = {
    generator = generators.toINI { };

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
