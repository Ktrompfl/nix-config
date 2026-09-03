{ config, lib, ... }:
with lib;
{
  services.fail2ban = {
    enable = lib.mkDefault config.services.openssh.enable;
    bantime = mkDefault "10m";
    bantime-increment = {
      enable = mkDefault true;
      factor = mkDefault "1";
      maxtime = mkDefault "48h";
      multipliers = mkDefault "1 2 4 8 16 32 64";
      rndtime = mkDefault "8m";
    };
    ignoreIP = [
      # local networks
      "10.0.0.0/8"
      "127.0.0.1/8"
      "172.16.0.0/12"
      "192.168.0.0/24"
    ];
    maxretry = mkDefault 5;
  };

  preservation.preserveAt.state-dir.directories = lib.optional config.services.fail2ban.enable "/var/lib/fail2ban";
}
