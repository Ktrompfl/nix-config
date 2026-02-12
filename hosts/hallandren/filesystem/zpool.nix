let
  # The filesystem / is stored on tmpfs and wiped on reboot.
  # The actual size for the tmpfs is usually much smaller than the assigned maximum.
  # Make sure to store large files on persisted data sets.
  root-size = "2G";

  # On ZFS, the performance will deteriorate significantly when more than 80% of the available space is used. To avoid this, reserve disk space beforehand.
  reservation-size = "10GiB";

  # The passphrase  for disk encryption must be provided for the initial installation.
  # This can either be provided interactively by setting the key location to "prompt"
  # or by providing a secret file.
  # For using the key with interactive login, the key can have no trailing newline.
  # For example use `echo -n "password" > /tmp/secret.key`.
  secret = "file:///tmp/secret.key";

  zpool = "pool0";
in
{

  disko.devices.zpool = {
    "${zpool}" = {
      type = "zpool";
      rootFsOptions = {
        mountpoint = "none";
        compression = "zstd";
        acltype = "posixacl";
        xattr = "sa";
        atime = "off";
        relatime = "off";
        encryption = "aes-256-gcm";
        keyformat = "passphrase";
        keylocation = secret;
      };
      postCreateHook = ''
        zfs set keylocation="prompt" $name;
      '';
      options.ashift = "12";
      datasets = {
        "root" = {
          type = "zfs_fs";
          options.mountpoint = "none";
        };
        "root/nix" = {
          type = "zfs_fs";
          mountpoint = "/nix";
          options = {
            atime = "off";
            canmount = "on";
            mountpoint = "/nix";
          };
        };
        "root/log" = {
          type = "zfs_fs";
          mountpoint = "/var/log";
          options.mountpoint = "/var/log";
        };
        "root/persist" = {
          # persisted data (marked for snapshots and backups)
          type = "zfs_fs";
          mountpoint = "/persist";
          options = {
            atime = "off";
            mountpoint = "/persist";
            "com.sun:auto-snapshot" = "true";
          };
        };
        "root/cache" = {
          # local data
          type = "zfs_fs";
          mountpoint = "/cache";
          options = {
            atime = "off";
            mountpoint = "/cache";
          };
        };
        "root/reserved" = {
          type = "zfs_fs";
          options = {
            mountpoint = "none";
            reservation = reservation-size;
          };
        };
      };
    };
  };

  # make sure the pool is found
  boot.zfs.extraPools = [ zpool ];

  # prompt for passphrase on boot
  boot.zfs.requestEncryptionCredentials = true;

  fileSystems."/" = {
    device = "none";
    fsType = "tmpfs";
    options = [
      "defaults"
      "size=${root-size}"
      "mode=755"
    ];
  };

  # mark preserved datasets as needed for boot
  fileSystems."/cache".neededForBoot = true;
  fileSystems."/persist".neededForBoot = true;
  fileSystems."/var/log".neededForBoot = true;

  ## setup preservation
  # preservation requires phase1 systemd
  boot.initrd.systemd.enable = true;
  preservation.enable = true;

  # Persisted data.
  # These directories and files represent important data that should
  # be backed up and preserved across system reinstalls.
  preservation.preserveAt.data-dir.persistentStoragePath = "/persist";

  # Machine-local state.
  # These directories represent mutable application or runtime state
  # that should persist across reboots but does not need to be backed up
  # or transferred between systems. This data is considered disposable.
  preservation.preserveAt.state-dir.persistentStoragePath = "/cache";
}
