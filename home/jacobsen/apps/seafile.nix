{ pkgs, ... }:
{
  apps.seafile-client = {
    package = pkgs.seafile-client;
    binaries = [ "seafile-applet" ];
    appId = "com.seafile.seafile-applet";

    access.Seafile = "rw";

    jail.permissions =
      c: with c; [
        desktop
        network
        notifications
        tray
      ];
  };

  preservation.preserveAt.data-dir.directories = [ "Seafile" ];
}
