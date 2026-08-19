{
  config,
  lib,
  pkgs,
  ...
}:
{
  packages = [ pkgs.fuzzel ];

  xdg.config.files."fuzzel/fuzzel.ini" = {
    generator = lib.generators.toINI { };
    value = {
      colors = with config.theme.colors; {
        background = opaque "background";
        text = opaque "foreground";
        prompt = opaque "foreground";
        input = opaque "foreground";
        placeholder = opaque "muted";
        match = opaque "highlight";
        counter = opaque "foreground";
        border = opaque "accent";
        selection = opaque "selection";
        "selection-text" = opaque "foreground";
        "selection-match" = opaque "highlight";
      };

      main = {
        font = "${config.theme.fonts.sansSerif.name}:size=${toString config.theme.fonts.sizes.popups}";
        icon-theme = config.theme.icons.name;
        layer = "overlay";
        launch-prefix = "${lib.getExe pkgs.runapp} --";
        horizontal-pad = 12;
        vertical-pad = 12;
        inner-pad = 8;
      };

      border = {
        width = 1;
        radius = 0;
      };
    };
  };
}
