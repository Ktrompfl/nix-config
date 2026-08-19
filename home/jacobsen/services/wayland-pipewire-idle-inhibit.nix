{
  graphicalService,
  lib,
  pkgs,
  ...
}:
let
  settings = (pkgs.formats.toml { }).generate "wayland-pipewire-idle-inhibit.toml" {
    verbosity = "INFO";
    media_minimum_duration = 5;
  };
in
{
  systemd.user.services.wayland-pipewire-idle-inhibit = graphicalService "background" {
    description = "Inhibit idle when audio is playing";
    serviceConfig.ExecStart = "${lib.getExe pkgs.wayland-pipewire-idle-inhibit} --config ${settings}";
  };
}
