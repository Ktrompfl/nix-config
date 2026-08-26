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

    # fallback to systemd to prompt for passphrase when plymouth fails
    services.systemd-ask-password-console.unitConfig.ConditionPathExists = "";
    paths.systemd-ask-password-console.unitConfig.ConditionPathExists = "";
  };

  preservation.preserveAt.state-dir.directories = lib.optional cfg.enable "/var/lib/plymouth";
}
