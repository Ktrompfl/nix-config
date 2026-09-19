{ pkgs, ... }:
{
  apps.chromium = {
    package = pkgs.chromium.override { enableWideVine = true; };
    appId = "org.chromium.Chromium";

    # downloads are written straight to disk; uploads go through the portal
    access.download = "rw";

    jail.permissions =
      c: with c; [
        desktop
        network
        notifications
      ];
  };
}
