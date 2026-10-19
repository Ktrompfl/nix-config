{
  config,
  inputs,
  pkgs,
  ...
}:
let
  stateDir = config.preservation.preserveAt.state-dir.persistentStoragePath;
in
{
  imports = [
    inputs.preservation.nixosModules.preservation
  ];

  # nix-mineral hardens these by bind mounting them onto themselves, which on a
  # tmpfs root only buys the mount options -- and costs a mount each, plus the
  # propagation that duplicated every preservation mount underneath. The root
  # filesystem carries the same options directly in the host's btrfs module.
  nix-mineral.filesystems.normal = {
    "/home".enable = false;
    "/var".enable = false;
    "/var/lib".enable = false;
    "/var/tmp".enable = false;
  };

  preservation.preserveAt.state-dir = {
    directories = [
      {
        directory = "/etc/nix";
        how = "symlink";
        createLinkTarget = true;
      }
      "/var/lib/hjem"
      {
        directory = "/var/lib/nixos";
        inInitrd = true;
      }
      "/var/lib/systemd"
    ];
    files = [
      {
        file = "/etc/adjtime";
        how = "symlink";
        configureParent = true;
      }
      {
        file = "/etc/machine-id";
        inInitrd = true;
      }
    ];
  };

  # The bind mount above is skipped while the persisted file does not exist, and
  # systemd then writes a fresh ID to the volatile /etc on every boot. Persist
  # that ID once, so the bind mount takes over from the next boot on.
  # (A symlink instead leaves systemd without any ID: it will not create the
  # file through a dangling link, and dbus and logind fail without it.)
  # systemd-tmpfiles skips every rule below a directory owned by someone other
  # than root ("unsafe path transition"), which silently disables preservation.
  systemd.tmpfiles.settings.preservation-root.${stateDir}.z = {
    user = "root";
    group = "root";
    mode = "0755";
  };

  systemd.suppressedSystemUnits = [ "systemd-machine-id-commit.service" ];
  systemd.services.preserve-machine-id = {
    description = "Persist the machine ID";
    wantedBy = [ "multi-user.target" ];
    unitConfig.ConditionPathExists = "!${stateDir}/etc/machine-id";
    serviceConfig = {
      Type = "oneshot";
      ExecStart = "${pkgs.coreutils}/bin/install -D -m 0444 /etc/machine-id ${stateDir}/etc/machine-id";
    };
  };
}
