{ helpers, ... }:
{
  imports = [
    ./engines.nix
    ./extensions.nix
    ./policies.nix
    ./preferences.nix
    ./user-chrome.nix
  ];

  files = {
    ".mozilla/firefox/profiles.ini" = {
      type = "copy";
      permissions = "0644";
      generator = helpers.generators.toMozillaProfiles;
      value.name = "default";
    };

    ".mozilla/firefox/default/containers.json" = {
      type = "copy";
      permissions = "0644";
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

  preservation.preserveAt.state-dir.directories = [ ".mozilla" ];

  environment.sessionVariables = {
    BROWSER = "firefox";
    DEFAULT_BROWSER = "firefox"; # for electron apps
  };
}
