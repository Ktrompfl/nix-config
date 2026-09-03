{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.programs.firefox;

  profilePath = ".mozilla/firefox/${cfg.profile.name}";

  mkSearch = pkgs.callPackage ./search.nix { firefox = cfg.package; };

  clobbered = lib.mapAttrs (_: file: file // { clobber = true; });

  extensionFiles = lib.listToAttrs (
    map (addon: {
      name = "${profilePath}/extensions/${addon.addonId}.xpi";
      value.source = "${addon}/share/mozilla/extensions/{ec8030f7-c20a-464f-9b0e-13a3a9e97384}/${addon.addonId}.xpi";
    }) cfg.profile.extensions.packages
  );

  extensionData = lib.mapAttrs' (
    addonId: settings:
    lib.nameValuePair "${profilePath}/browser-extension-data/${addonId}/storage.js" {
      generator = (pkgs.formats.json { }).generate "${addonId}-storage.js";
      value = settings;
    }
  ) cfg.profile.extensions.settings;
in
{
  options.programs.firefox = {
    enable = lib.mkEnableOption "Firefox";

    package = lib.mkOption {
      type = lib.types.package;
      default = pkgs.firefox;
      description = "Used to read the application name the search hash is salted with.";
    };

    profile = {
      name = lib.mkOption {
        type = lib.types.str;
        default = "default";
      };

      settings = lib.mkOption {
        type =
          with lib.types;
          attrsOf (oneOf [
            bool
            int
            str
            attrs
            (listOf anything)
          ]);
        default = { };
        description = ''
          Preferences written to the profile's user.js. Attribute sets and
          lists are encoded as JSON strings, which is how Firefox stores them.
        '';
      };

      userChrome = lib.mkOption {
        type = lib.types.lines;
        default = "";
      };

      containers = lib.mkOption {
        type = lib.types.attrs;
        default = { };
        description = "Contextual identities, keyed by name.";
      };

      extensions = {
        packages = lib.mkOption {
          type = lib.types.listOf lib.types.package;
          default = [ ];
        };

        settings = lib.mkOption {
          type = lib.types.attrsOf lib.types.attrs;
          default = { };
          description = "Extension storage, keyed by addon id.";
        };
      };

      search = {
        engines = lib.mkOption {
          type = lib.types.attrs;
          default = { };
        };

        default = lib.mkOption {
          type = lib.types.nullOr lib.types.str;
          default = null;
        };

        order = lib.mkOption {
          type = lib.types.listOf lib.types.str;
          default = [ ];
        };
      };
    };
  };

  config = lib.mkIf cfg.enable {
    files = clobbered (
      {
        ".mozilla/firefox/profiles.ini" = {
          generator = lib.generators.toMozillaProfiles;
          value.name = cfg.profile.name;
        };

        "${profilePath}/user.js" = {
          generator = lib.generators.toMozillaPrefs;

          value = {
            # Extension storage has to stay in the flat JSON backend for the
            # per-extension storage.js files below to be read at all.
            "extensions.webextensions.ExtensionStorageIDB.enabled" = false;
          }
          // lib.optionalAttrs (cfg.profile.userChrome != "") {
            # userChrome.css is ignored without this.
            "toolkit.legacyUserProfileCustomizations.stylesheets" = true;
          }
          // cfg.profile.settings;
        };

        "${profilePath}/containers.json".source =
          let
            # Firefox keeps two private contexts of its own in this file and
            # recreates them if they are missing, taking ownership of it.
            internal =
              map
                (context: {
                  accessKey = "";
                  color = "";
                  icon = "";
                  public = false;
                  inherit (context) name userContextId;
                })
                [
                  {
                    name = "userContextIdInternal.thumbnail";
                    userContextId = 4294967294;
                  }
                  {
                    name = "userContextIdInternal.webextStorageLocal";
                    userContextId = 4294967295;
                  }
                ];

            ours = lib.mapAttrsToList (name: container: {
              userContextId = container.id;
              public = true;
              inherit (container) icon color;
              name = container.name or name;
            }) cfg.profile.containers;
          in
          # Written compact, and as version 5, which is what Firefox writes
          # itself; a pretty-printed file is rewritten on first start.
          pkgs.writeText "containers.json" (
            builtins.toJSON {
              version = 5;
              # The highest id handed out so far, not a count: Firefox allocates
              # the next container from it.
              lastUserContextId = lib.foldl' lib.max 0 (lib.mapAttrsToList (_: c: c.id) cfg.profile.containers);
              identities = ours ++ internal;
            }
          );
      }
      // lib.optionalAttrs (cfg.profile.userChrome != "") {
        "${profilePath}/chrome/userChrome.css".text = cfg.profile.userChrome;
      }
      // lib.optionalAttrs (cfg.profile.search.engines != { }) {
        "${profilePath}/search.json.mozlz4".source = mkSearch {
          profilePath = cfg.profile.name;
          inherit (cfg.profile.search) engines default order;
        };
      }
      // extensionFiles
      // extensionData
    );
  };
}
