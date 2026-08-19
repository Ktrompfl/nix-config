{
  services.btrfs.autoScrub = {
    enable = true;
    interval = "monthly";
  };

  services.snapper.configs.persist = {
    FSTYPE = "btrfs";
    SUBVOLUME = "/persist";
    TIMELINE_CREATE = true;
    TIMELINE_CLEANUP = true;
    TIMELINE_LIMIT_HOURLY = 24;
    TIMELINE_LIMIT_DAILY = 7;
    TIMELINE_LIMIT_WEEKLY = 4;
    TIMELINE_LIMIT_MONTHLY = 0;
  };

  fileSystems."/" = {
    device = "none";
    fsType = "tmpfs";
    options = [
      "defaults"
      "size=2G"
      "mode=755"
    ];
  };

  # mark preserved datasets as needed for boot
  fileSystems."/cache".neededForBoot = true;
  fileSystems."/persist".neededForBoot = true;
  fileSystems."/var/log".neededForBoot = true;

  # nix-mineral merges `bind` into the options of every mount point it hardens,
  # where it wins over `device` and `subvol`. That turns /var/log into a self
  # bind mount of the root tmpfs instead of the `log` subvolume above, so the
  # journal does not survive a reboot. Drop the bind, keep nosuid/noexec/nodev.
  nix-mineral.filesystems.normal."/var/log".options.bind = false;

  services.journald.extraConfig = "SystemMaxUse=10G";

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
