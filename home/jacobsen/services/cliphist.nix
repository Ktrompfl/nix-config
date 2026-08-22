{
  graphicalService,
  lib,
  pkgs,
  ...
}:
let
  wl-paste = lib.getExe' pkgs.wl-clipboard "wl-paste";
  store = "${lib.getExe pkgs.cliphist} -max-dedupe-search 10 -max-items 500 store";
in
{
  packages = [ pkgs.cliphist ];

  systemd.services = {
    cliphist = graphicalService "background" {
      description = "Clipboard management daemon";
      serviceConfig.ExecStart = "${wl-paste} --watch ${store}";
    };

    cliphist-images = graphicalService "background" {
      description = "Clipboard management daemon for images";
      serviceConfig.ExecStart = "${wl-paste} --type image --watch ${store}";
    };
  };
}
