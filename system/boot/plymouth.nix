{ config, lib, ... }:
let
  cfg = config.boot.plymouth;
in
{
  boot.kernelParams = lib.optionals cfg.enable [
    "quiet"
    "splash"
  ];

  boot.initrd.systemd = lib.mkIf cfg.enable {
    # start plymouth after amdgpu is loaded in initrd
    services.plymouth-start.after = [ "systemd-modules-load.service" ];
  };

  preservation.preserveAt.state-dir.directories = lib.optional cfg.enable {
    directory = "/var/lib/plymouth";
    how = "symlink";
    createLinkTarget = true;
  };
}
