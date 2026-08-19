{ inputs, ... }:
{
  imports = [
    inputs.preservation.nixosModules.preservation
  ];

  # nix-mineral binds mounts the following onto themselves;
  # hardens /home, /var, /var/lib and /var/tmp by bind mounting them
  # onto themselves. Those are plain directories on the root tmpfs here, so the
  # new mount joins the same propagation peer group as /, and every preservation
  # mount underneath ends up in the kernel mount table two to four times. On
  # shutdown systemd then fails to unmount the copies it has no unit for.
  # Making the hardening mounts private stops mounts below them from propagating.
  nix-mineral.filesystems.normal = {
    "/home".options.private = true;
    "/var".options.private = true;
    "/var/lib".options.private = true;
    "/var/tmp".options.private = true;
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
