{
  config,
  lib,
  ...
}:
let
  inherit (lib)
    mapAttrs
    mkDefault
    mkOption
    types
    ;

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

        serviceConfig = {
          Type = mkDefault "simple";
          Restart = mkDefault "on-failure";
          Slice = mkDefault "${config.slice}-graphical.slice";
        };
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
      graphical-session.target and the slice.
    '';
  };

  config.systemd.services = mapAttrs (
    _: service: removeAttrs service [ "slice" ]
  ) config.wayland.services;
}
