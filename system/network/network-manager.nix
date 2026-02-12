{ config, lib, ... }:
let
  cfg = config.networking.networkmanager;
in
{
  users.users.jacobsen.extraGroups = lib.optional cfg.enable "networkmanager";

  preservation.preserveAt.state-dir.directories = lib.optionals cfg.enable [
    "/etc/NetworkManager/system-connections"
    "/var/lib/NetworkManager"
  ];
}
