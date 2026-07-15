{ inputs, ... }:
let
  disk0 = "/dev/nvme0n1";
  boot-size = "1G";
  swap-size = "20G";
in
{
  imports = [ inputs.disko.nixosModules.disko ];

  disko.devices.disk.disk0 = {
    type = "disk";
    device = disk0;
    content = {
      type = "gpt";
      partitions = {
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
        crypt0 = {
          size = "100%";
          content = {
            type = "luks";
            name = "crypt0";
            settings.allowDiscards = true;
            content = {
              type = "lvm_pv";
              vg = "vg0";
            };
          };
        };
      };
    };
  };

  disko.devices.lvm_vg.vg0 = {
    type = "lvm_vg";
    lvs = {
      swap = {
        size = swap-size;
        content = {
          type = "swap";
          resumeDevice = true;
          discardPolicy = "both";
        };
      };
      root = {
        size = "100%";
        content = {
          type = "btrfs";
          extraArgs = [ "-f" ];
          subvolumes = {
            "/nix" = {
              mountpoint = "/nix";
              mountOptions = [
                "compress=zstd"
                "noatime"
              ];
            };
            "/persist" = {
              mountpoint = "/persist";
              mountOptions = [
                "compress=zstd"
                "noatime"
              ];
            };
            "/log" = {
              mountpoint = "/var/log";
              mountOptions = [
                "compress=zstd"
                "noatime"
              ];
            };
            "/cache" = {
              mountpoint = "/cache";
              mountOptions = [
                "compress=zstd"
                "noatime"
              ];
            };
          };
        };
      };
    };
  };
}
