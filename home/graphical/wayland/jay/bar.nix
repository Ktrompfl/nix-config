{
  lib,
  pkgs,
  helpers,
  ...
}:
{
  xdg.config.files."jay/config.toml".value = {
    show-bar = true;

    status = {
      format = "i3bar";
      exec = [
        (lib.getExe' pkgs.i3status-rust "i3status-rs")
        "config-jay.toml"
      ];
      i3bar-separator = "";
    };

    on-graphics-initialized = [ helpers.jay.bar.init ];
  };
}
