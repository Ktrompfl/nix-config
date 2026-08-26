{
  imports = [
    ./plymouth.nix
  ];

  # longer timeout before luks decryption fails and drops into emergency shell
  boot.initrd.systemd.settings.Manager.DefaultDeviceTimeoutSec = "5min";
}
