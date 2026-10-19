{
  helpers,
  lib,
  pkgs,
  ...
}:
{
  systemd.targets.jay-session = {
    description = "jay session with initialized graphics";
    bindsTo = [ "graphical-session.target" ];
    after = [ "graphical-session.target" ];
  };

  xdg.config.files."jay/config.toml".value = {
    # Can only be set in one place, so the bar's initialization runs from here
    # too.
    on-graphics-initialized = [
      helpers.jay.bar.init
      (helpers.jay.exec [
        (lib.getExe' pkgs.systemd "systemctl")
        "--user"
        "start"
        "jay-session.target"
      ])
    ];
  };
}
