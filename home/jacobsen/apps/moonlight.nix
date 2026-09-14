{ pkgs, ... }:
{
  apps.moonlight-qt = {
    package = pkgs.moonlight-qt; # remote play
    binaries = [ "moonlight" ];
    appId = "com.moonlight_stream.Moonlight";

    jail.permissions =
      c: with c; [
        desktop
        network
      ];
  };
}
