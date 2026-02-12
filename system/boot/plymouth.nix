{ config, lib, ... }:
let
  cfg = config.boot.plymouth;
in
{
  boot.kernelParams = lib.optionals cfg.enable [
    "quiet"
    "splash"
  ];

  preservation.preserveAt.state-dir.directories = lib.optional cfg.enable "/var/lib/plymouth";
}
