{ inputs, ... }:
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
      "/etc/nix"
      {
        directory = "/var/lib/nixos";
        inInitrd = true;
      }
      "/var/lib/systemd"
    ];
    files = [ "/etc/adjtime" ];
  };
}
