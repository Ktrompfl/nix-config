{ config, lib, ... }:
let
  cfg = config.networking.networkmanager;
in
lib.mkIf cfg.enable {
  users.users.jacobsen.extraGroups = "networkmanager";

  preservation.preserveAt.state-dir.directories = [
    "/etc/NetworkManager/system-connections"
    "/var/lib/NetworkManager"
  ];

  networking.networkmanager = {
    ethernet.macAddress = "preserve";
  };
}
