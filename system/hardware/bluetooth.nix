{ config, lib, ... }:
let
  cfg = config.hardware.bluetooth;
in
{
  hardware.bluetooth.powerOnBoot = lib.mkDefault false;

  preservation.preserveAt.state-dir.directories = lib.optional cfg.enable "/var/lib/bluetooth";
}
