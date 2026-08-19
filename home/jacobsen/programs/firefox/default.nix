{
  config,
  lib,
  pkgs,
  ...
}:
{
  programs.firefox = {
    enable = true;

    profile = {
      name = "default";
      settings = import ./preferences.nix { inherit config; };
      userChrome = import ./userChrome.nix null;
      extensions =
        let
          declared = import ./extensions.nix { inherit config pkgs; };
        in
        {
          inherit (declared) packages;
          settings = lib.mapAttrs (_: e: e.settings or { }) (
            lib.filterAttrs (_: e: e ? settings) declared.settings
          );
        };

      search = {
        engines = import ./engines.nix { inherit pkgs; };
        default = "google";
        order = [ "google" ];
      };

      containers = {
        private = {
          id = 1;
          color = "blue";
          icon = "fingerprint";
        };
        science = {
          id = 2;
          color = "turquoise";
          icon = "circle";
        };
        develop = {
          id = 3;
          color = "orange";
          icon = "briefcase";
        };
      };
    };
  };

  environment.sessionVariables = {
    BROWSER = lib.getExe pkgs.firefox;
    DEFAULT_BROWSER = lib.getExe pkgs.firefox; # for electron apps
  };

  preservation.preserveAt.state-dir.directories = [
    ".config/mozilla/firefox"
    ".mozilla"
  ];
}
