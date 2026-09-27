{ config, lib, ... }:
let
  cfg = config.services.openssh;
in
{
  services.openssh = {
    enable = lib.mkDefault true;
    openFirewall = true;
    settings = {
      # securing the ssh server
      AllowUsers = [ "jacobsen" ];
      KbdInteractiveAuthentication = false;
      PasswordAuthentication = false;
      PermitRootLogin = "no";
      TCPKeepAlive = false;
      PermitEmptyPasswords = false;
      PermitTunnel = false;
      # UseDns = false;
      MaxAuthTries = 3;
      MaxSessions = 2;
      ClientAliveInterval = 300;
      ClientAliveCountMax = 0;
      AllowTcpForwarding = false;
      AllowAgentForwarding = false;
      X11Forwarding = false;
      LogLevel = "VERBOSE";
    };
  };

  # Where home/common/programs/ssh.nix keeps the user's own keys, on top of
  # the default below the home directory, which is temporary.
  services.openssh.authorizedKeysFiles = [
    "${config.preservation.preserveAt.data-dir.persistentStoragePath}%h/.local/share/ssh/authorized_keys"
  ];

  # preserve host keys
  preservation.preserveAt.data-dir.files = lib.optionals cfg.enable (
    lib.concatMap (key: [
      {
        file = key.path;
        how = "symlink";
        configureParent = true;
      }
      {
        file = "${key.path}.pub";
        how = "symlink";
        configureParent = true;
      }
    ]) cfg.hostKeys
  );
}
