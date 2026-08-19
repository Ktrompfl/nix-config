{
  graphicalService,
  lib,
  pkgs,
  ...
}:
{
  # keeps clipboard contents after the source window closes
  systemd.user.services.wl-clip-persist = graphicalService "background" {
    description = "Wayland clipboard persistence daemon";
    serviceConfig.ExecStart = "${lib.getExe pkgs.wl-clip-persist} --clipboard regular";
  };
}
