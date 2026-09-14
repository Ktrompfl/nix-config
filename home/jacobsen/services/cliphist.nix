{
  config,
  lib,
  pkgs,
  sandboxedService,
  ...
}:
let
  wl-paste = lib.getExe' pkgs.wl-clipboard "wl-paste";
  store = "${lib.getExe pkgs.cliphist} -max-dedupe-search 10 -max-items 500 store";

  # cliphist keeps its database below the xdg cache directory, which the
  # default-deny home withholds
  database.BindPaths = [ "${config.directory}/.local/cache" ];
in
{
  packages = [ pkgs.cliphist ];

  systemd.services = {
    cliphist = sandboxedService "background" {
      description = "Clipboard management daemon";
      serviceConfig = {
        ExecStart = "${wl-paste} --watch ${store}";
        inherit (database) BindPaths;
      };
    };

    cliphist-images = sandboxedService "background" {
      description = "Clipboard management daemon for images";
      serviceConfig = {
        ExecStart = "${wl-paste} --type image --watch ${store}";
        inherit (database) BindPaths;
      };
    };
  };
}
