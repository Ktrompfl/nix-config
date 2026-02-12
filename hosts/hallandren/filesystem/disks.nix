let
  disk0 = "/dev/nvme0n1";
  disk1 = "/dev/nvme1n1";

  # On NixOS, the bootloader, kernel and initrd of multiple generations are stored on the boot partition (ESP).
  # Therefore, reserving at least 1GiB on the boot partition is recommended.
  boot-size = "1G";

  # ZFS does not allow to use swapfiles, but users can use a ZFS volume (ZVOL) as swap.
  # However, this comes with various problems:
  # - On systems with extremely high memory pressure, using a zvol for swap can result in lockup, regardless of how much swap is still available.
  # - Swap on zvol does not support resume from hibernation, attempt to resume will result in pool corruption.
  # Therefore, an external swap partition with fixed size is used.
  swap-size = "16G";

  zpool = "pool0";

  swap-partition = {
    size = swap-size;
    content = {
      type = "swap";
      randomEncryption = true;
    };
  };

  zfs-partition = {
    size = "100%";
    content = {
      type = "zfs";
      pool = zpool;
    };
  };
in
{
  disko.devices.disk = {
    disk0 = {
      type = "disk";
      device = disk0;
      content = {
        type = "gpt";
        partitions = {
          # boot partition only on one disk
          ESP = {
            size = boot-size;
            type = "EF00";
            content = {
              type = "filesystem";
              format = "vfat";
              mountpoint = "/boot";
              mountOptions = [
                "nofail"
                "umask=0077"
              ];
            };
          };

          # primary swap partition
          swap = swap-partition;

          # zfs partitions assigned to one pool
          zfs = zfs-partition;
        };
      };
    };
    disk1 = {
      type = "disk";
      device = disk1;
      content = {
        type = "gpt";
        partitions = {
          # secondary swap partition
          swap = swap-partition;

          # zfs partitions assigned to one pool
          zfs = zfs-partition;
        };
      };
    };
  };
}
