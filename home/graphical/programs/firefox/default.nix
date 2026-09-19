{
  helpers,
  pkgs,
  ...
}:
{
  imports = [
    ./engines.nix
    ./extensions.nix
    ./preferences.nix
    ./user-chrome.nix
  ];

  apps.firefox = {
    package = pkgs.firefox;
    appId = "org.mozilla.firefox";

    # downloads are written straight to disk; uploads go through the portal
    access.download = "rw";

    jail.permissions =
      c: with c; [
        desktop
        network
        notifications
      ];

    files = {
      ".mozilla/firefox/profiles.ini" = {
        generator = helpers.generators.toMozillaProfiles;
        value.name = "default";
      };

      ".mozilla/firefox/default/containers.json" = {
        generator = helpers.generators.toFirefoxContainers;
        value = {
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
  };

  environment.sessionVariables = {
    BROWSER = "firefox";
    DEFAULT_BROWSER = "firefox"; # for electron apps
  };
}
