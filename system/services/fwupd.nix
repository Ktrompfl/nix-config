{ config, lib, ... }:
let
  cfg = config.services.fwupd;
in
{
  preservation.preserveAt.state-dir.directories = lib.optional cfg.enable "/var/lib/fwupd";
}
