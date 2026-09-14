{
  sandboxedService,
  lib,
  pkgs,
  ...
}:
{
  packages = [ pkgs.wl-clip-persist ];

  systemd.services.wl-clip-persist = sandboxedService "background" {
    description = "Wayland clipboard persistence daemon";
    serviceConfig.ExecStart = "${lib.getExe pkgs.wl-clip-persist} --clipboard regular";
  };
}
