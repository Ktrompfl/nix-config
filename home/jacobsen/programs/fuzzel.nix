{
  lib,
  pkgs,
  ...
}:
{
  programs.fuzzel = {
    enable = true;
    settings = {
      main = {
        layer = "overlay";
        launch-prefix = "${lib.getExe pkgs.app2unit} --";
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
