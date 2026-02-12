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
    daemonSettings = {
      Definition = {
        loglevel = mkDefault "INFO";
        logtarget = "/var/log/fail2ban/fail2ban.log";
        socket = "/run/fail2ban/fail2ban.sock";
        pidfile = "/run/fail2ban/fail2ban.pid";
        dbfile = "/var/lib/fail2ban/fail2ban.sqlite3";
        dbpurageage = mkDefault "1d";
      };
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

  services.logrotate.settings."/var/log/fail2ban/fail2ban.log" = { };

  preservation.preserveAt.state-dir.directories = lib.optional config.services.fail2ban.enable "/var/lib/fail2ban";
}
