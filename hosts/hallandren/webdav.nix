{ config, ... }:
let
  uid = toString config.users.users.jacobsen.uid;
  dir = "/home/jacobsen/webdav";
in
{
  services.davfs2.enable = true;

  # systemd user mounts don't work with davfs2
  systemd.mounts = [
    {
      enable = true;
      description = "Mount WebDAV Service for AStA Cloud";
      after = [ "network-online.target" ];
      wants = [ "network-online.target" ];
      what = "https://cloud.asta.uni-kl.de/remote.php/dav/files/jacobsen";
      where = "${dir}/asta";
      options = "uid=${uid},file_mode=0664,dir_mode=2775,grpid";
      type = "davfs";
      mountConfig.TimeoutSec = 15;
    }
  ];

  systemd.automounts = [
    {
      description = "Mount WebDAV Service for AStA Cloud";
      wantedBy = [ "multi-user.target" ];
      where = "${dir}/asta";
      automountConfig.TimeoutIdleSec = 120;
    }
  ];
}
