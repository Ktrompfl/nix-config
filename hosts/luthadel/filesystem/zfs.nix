{
  config,
  lib,
  pkgs,
  ...
}:
{
  boot = {
    # use the latest ZFS-compatible Kernel currently available by default
    # note: this might jump back and forth as kernels are added or removed.
    kernelPackages =
      let
        zfsCompatibleKernelPackages = lib.filterAttrs (
          name: kernelPackages:
          (builtins.match "linux_[0-9]+_[0-9]+" name) != null
          && (builtins.tryEval kernelPackages).success
          && (!kernelPackages.${config.boot.zfs.package.kernelModuleAttribute}.meta.broken)
        ) pkgs.linuxKernel.packages;
        latestKernelPackage = lib.last (
          lib.sort (a: b: (lib.versionOlder a.kernel.version b.kernel.version)) (
            builtins.attrValues zfsCompatibleKernelPackages
          )
        );
      in
      lib.mkDefault latestKernelPackage;

    # enable support for zfs
    supportedFilesystems = [ "zfs" ];
    initrd.supportedFilesystems = [ "zfs" ];
  };

  # enable periodic scrubbing of all ZFS pools
  services.zfs.autoScrub = {
    enable = true;
    interval = "monthly";
  };

  # enable periodic auto-snapshotting of ZFS pools marked with 'com.sun:auto-snapshot'
  services.zfs.autoSnapshot = {
    enable = true;
    frequent = 4; # 15 min
    hourly = 24;
    daily = 7;
    weekly = 4;
    monthly = 0;
  };

  # enable periodic TRIM on all ZFS pools
  services.zfs.trim = {
    enable = true; # enable periodic TRIM on all ZFS pools
    interval = "weekly";
  };

  preservation.preserveAt.state-dir.files = [
    {
      file = "/etc/zfs/zpool.cache";
      inInitrd = true;
    }
  ];
}
