{ pkgs, ... }:
{
  apps.signal-desktop = {
    package = pkgs.signal-desktop;
    appId = "org.signal.Signal";

    # attachments are saved straight to disk rather than through the portal
    access.download = "rw";

    jail.permissions =
      c: with c; [
        desktop
        network
        notifications
      ];
  };
}
