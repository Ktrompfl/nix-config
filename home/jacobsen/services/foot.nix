{ pkgs, ... }:
{
  systemd.packages = [ pkgs.foot ];

  systemd.user.sockets.foot-server.wantedBy = [ "graphical-session.target" ];

  systemd.user.services.foot-server.serviceConfig.Slice = "app-graphical.slice";
}
