{
  config,
  lib,
  ...
}:
let
  inherit (lib)
    mapAttrs
    mkDefault
    mkIf
    mkMerge
    mkOption
    types
    ;

  homeDirectory = config.directory;

  serviceModule =
    { config, ... }:
    {
      freeformType = types.attrsOf types.raw;

      options = {
        slice = mkOption {
          type = types.enum [
            "background"
            "session"
          ];
          default = "background";
          description = "Which of the graphical slices the unit is accounted to.";
        };

        sandbox = mkOption {
          type = types.bool;
          default = true;
          description = ''
            Whether the hardening below applies. It is the default because
            these daemons want the wayland socket and almost nothing else;
            turn it off only for one that cannot live inside it at all, and
            say why.
          '';
        };

        serviceConfig = mkOption {
          type = types.attrsOf types.raw;
          default = { };
          description = "The unit's [Service] section.";
        };
      };

      config = {
        partOf = mkDefault [ "graphical-session.target" ];
        after = mkDefault [ "graphical-session.target" ];
        wantedBy = mkDefault [ "graphical-session.target" ];

        serviceConfig = mkMerge [
          {
            Type = mkDefault "simple";
            Restart = mkDefault "on-failure";
            Slice = mkDefault "${config.slice}-graphical.slice";
          }
          (mkIf config.sandbox {
            TemporaryFileSystem = mkDefault "${homeDirectory}:ro";

            NoNewPrivileges = mkDefault true;
            PrivateTmp = mkDefault true;
            ProtectSystem = mkDefault "full";
            ProtectKernelTunables = mkDefault true;
            ProtectKernelModules = mkDefault true;
            ProtectKernelLogs = mkDefault true;
            ProtectControlGroups = mkDefault true;
            ProtectClock = mkDefault true;
            ProtectHostname = mkDefault true;
            RestrictNamespaces = mkDefault true;
            RestrictRealtime = mkDefault true;
            RestrictSUIDSGID = mkDefault true;
            LockPersonality = mkDefault true;

            RestrictAddressFamilies = mkDefault [ "AF_UNIX" ];

            SystemCallArchitectures = mkDefault "native";
            SystemCallFilter = mkDefault [
              "@system-service"
              "~@privileged"
              "~@resources"
            ];

            UMask = mkDefault "0077";
          })
        ];
      };
    };
in
{
  options.wayland.services = mkOption {
    type = types.attrsOf (types.submodule serviceModule);
    default = { };
    description = ''
      Units that belong to the graphical session. Each is an ordinary systemd
      service definition; what this adds is the ordering against
      graphical-session.target, the slice, and the sandbox.
    '';
  };

  config.systemd.services = mapAttrs (
    _: service:
    removeAttrs service [
      "slice"
      "sandbox"
    ]
  ) config.wayland.services;
}
