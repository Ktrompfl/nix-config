{
  graphicalService,
  lib,
  pkgs,
  ...
}:
{
  # notifications fail silently if this is killed
  systemd.user.services.swaync = graphicalService "session" {
    description = "Swaync notification daemon";
    serviceConfig.ExecStart = lib.getExe' pkgs.swaynotificationcenter "swaync";
  };
}
