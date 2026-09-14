{ config, lib, ... }:
let
  graphicalService =
    slice: unit:
    lib.recursiveUpdate {
      partOf = [ "graphical-session.target" ];
      after = [ "graphical-session.target" ];
      wantedBy = [ "graphical-session.target" ];

      serviceConfig = {
        Type = "simple";
        Restart = "on-failure";
        Slice = "${slice}-graphical.slice";
      };
    } unit;
in
{
  imports = [
    ./awww.nix
    ./cliphist.nix
    ./foot.nix
    ./swayidle.nix
    ./swaync
    ./wayland-pipewire-idle-inhibit.nix
    ./wl-clip-persist.nix
    ./wl-tray-bridge.nix
  ];

  _module.args.graphicalService = graphicalService;

  # The same default-deny shape the jailed applications get, expressed in the
  # unit rather than in a wrapper. These daemons want the wayland socket and
  # almost nothing else, and systemd already owns the namespaces and the
  # seccomp filter, so bubblewrap would only add a process and a shell script
  # between systemd and the thing it is supervising.
  #
  # Deliberately absent:
  #
  #   ProtectHome=    for a user service this also remounts $XDG_RUNTIME_DIR
  #                   read-only, which stops any of these from creating its
  #                   own socket there. TemporaryFileSystem= below withholds
  #                   the home directory without that side effect, and grants
  #                   go back in per service with Bind{,ReadOnly}Paths=.
  #
  #   ProtectSystem=strict
  #                   same problem, for the same directory.
  #
  #   PrivateDevices= would take /dev/dri with it.
  _module.args.sandboxedService =
    slice: unit:
    lib.recursiveUpdate (graphicalService slice {
      serviceConfig = {
        TemporaryFileSystem = "${config.directory}:ro";

        NoNewPrivileges = true;
        PrivateTmp = true;
        ProtectSystem = "full";
        ProtectKernelTunables = true;
        ProtectKernelModules = true;
        ProtectKernelLogs = true;
        ProtectControlGroups = true;
        ProtectClock = true;
        ProtectHostname = true;
        RestrictNamespaces = true;
        RestrictRealtime = true;
        RestrictSUIDSGID = true;
        LockPersonality = true;

        # none of these speak to anything but the compositor and the bus
        RestrictAddressFamilies = [ "AF_UNIX" ];

        SystemCallArchitectures = "native";
        SystemCallFilter = [
          "@system-service"
          "~@privileged"
          "~@resources"
        ];

        UMask = "0077";
      };
    }) unit;
}
