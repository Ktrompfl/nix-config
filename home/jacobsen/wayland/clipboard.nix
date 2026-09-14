{
  config,
  lib,
  pkgs,
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
  packages = [
    pkgs.cliphist
    pkgs.wl-clipboard
    pkgs.wl-clip-persist
  ];

  wayland.services = {
    cliphist = {
      description = "Clipboard management daemon";
      serviceConfig = {
        ExecStart = "${wl-paste} --watch ${store}";
        inherit (database) BindPaths;
      };
    };

    cliphist-images = {
      description = "Clipboard management daemon for images";
      serviceConfig = {
        ExecStart = "${wl-paste} --type image --watch ${store}";
        inherit (database) BindPaths;
      };
    };

    wl-clip-persist = {
      description = "Wayland clipboard persistence daemon";
      serviceConfig.ExecStart = "${lib.getExe pkgs.wl-clip-persist} --clipboard regular";
    };
  };
}
