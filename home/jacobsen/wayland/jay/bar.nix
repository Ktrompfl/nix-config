{
  jayLib,
  lib,
  pkgs,
  ...
}:
{
  show-bar = true;

  # Everything that is shown in the bar is rendered by i3status-rust, which
  # decides icons, thresholds, severity colors, and which blocks to hide; see
  # ../../i3status-rust.nix. Jay only concatenates the blocks it
  # prints, which is why the separator is empty: every block brings its own
  # padding.
  status = {
    format = "i3bar";
    exec = [
      (lib.getExe' pkgs.i3status-rust "i3status-rs")
      "config-jay.toml"
    ];
    i3bar-separator = "";
  };

  on-graphics-initialized = [ jayLib.bar.init ];
}
