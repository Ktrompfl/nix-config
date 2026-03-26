{ config, pkgs, ... }:
let
  uid = toString config.users.users.jacobsen.uid;
  dir = "/home/jacobsen/nas";
  credentials = "/etc/samba/credentials";
in
{
  # The cifs kernel modules are disabled by the kicksecure module blacklist.
  nix-mineral.settings.etc.kicksecure-module-blacklist = false;
  boot.supportedFilesystems.cifs = true;

  environment.systemPackages = with pkgs; [
    cifs-utils
    keyutils
    samba # TODO: remove
  ];

  systemd.mounts = [
    {
      enable = true;
      description = "Mount Share Service for AStA NAS";
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      what = "//fp0.uni-kl.de/DFS/tu/asta";
      where = "${dir}/asta";
      options = "_netdev,iocharset=utf8,rw,credentials=${credentials}/asta,uid=${uid},gid=100";
      type = "cifs";
      mountConfig.TimeoutSec = 15;
    }
  ];

  systemd.automounts = [
    {
      description = "Mount Share Service for AStA NAS";
      wantedBy = [ "multi-user.target" ];
      where = "${dir}/asta";
      automountConfig.TimeoutIdleSec = 120;
    }
  ];

  sops.secrets."samba/asta" = {
    mode = "0600";
    path = "${credentials}/asta";
  };
}
