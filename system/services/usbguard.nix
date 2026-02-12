{ config, lib, ... }:
let
  cfg = config.services.usbguard;
in
{
  services.usbguard = {
    enable = true;
    dbus.enable = true;
    presentDevicePolicy = "allow"; # Automatically allow all connected devices at boot in USBGuard.
    IPCAllowedGroups = [ "wheel" ];
  };

  preservation.preserveAt.state-dir.directories = lib.optional cfg.enable "/var/lib/usbguard";
}
