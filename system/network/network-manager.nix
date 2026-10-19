{ config, lib, ... }:
let
  cfg = config.networking.networkmanager;
in
lib.mkIf cfg.enable {
  users.users.jacobsen.extraGroups = [ "networkmanager" ];

  preservation.preserveAt.state-dir.directories = [
    {
      directory = "/etc/NetworkManager/system-connections";
      mode = "0700";
    }
    "/var/lib/NetworkManager"
  ];

  networking.networkmanager = {
    ethernet.macAddress = "preserve";
  };
}
