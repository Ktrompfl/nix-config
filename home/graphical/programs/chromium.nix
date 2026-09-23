{ config, pkgs, ... }:
let
  policies = "/etc/chromium/policies/managed/extra.json";

  extra = (pkgs.formats.json { }).generate "chromium-policies.json" {
    BrowserThemeColor = config.theme.colors.withHashtag.base00;
  };
in
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

        (ro-bind "${extra}" policies)
      ];
  };
}
