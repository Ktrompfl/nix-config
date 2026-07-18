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
}
