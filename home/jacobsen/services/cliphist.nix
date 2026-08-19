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
  # clipboard history; the clipboard itself keeps working
  systemd.user.services = {
    cliphist = graphicalService "background" {
      description = "Clipboard management daemon";
      serviceConfig.ExecStart = "${wl-paste} --watch ${store}";
    };

    # Images are watched separately; wl-paste can only follow one mime type.
    cliphist-images = graphicalService "background" {
      description = "Clipboard management daemon for images";
      serviceConfig.ExecStart = "${wl-paste} --type image --watch ${store}";
    };
  };
}
