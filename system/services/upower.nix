{ config, lib, ... }:
let
  cfg = config.services.upower;
in
{
  preservation.preserveAt.state-dir.directories = lib.optional cfg.enable "/var/lib/upower";
}
