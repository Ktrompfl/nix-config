{
  config,
  helpers,
  pkgs,
  ...
}:
let
  # ssh only ever reads its configuration from ~/.ssh, but everything else is
  # named here: the keys and known hosts on the backed up partition, the
  # control sockets in the runtime directory.
  directory = "${config.storage.data}/.local/share/ssh";

  # Every one named has to exist, or ssh complains on each connection.
  identities = [ "id_ed25519" ];

  hosts = {
    "*" = {
      ForwardAgent = false;
      AddKeysToAgent = "no";
      Compression = false;
      ServerAliveInterval = 15;
      ServerAliveCountMax = 3;
      HashKnownHosts = false;
      IdentityFile = map (identity: "${directory}/${identity}") identities;
      UserKnownHostsFile = "${directory}/known_hosts";
      ControlMaster = "auto";
      ControlPath = "\${XDG_RUNTIME_DIR}/ssh-%C";
      ControlPersist = "5m";
    };
    fsmathe = {
      HostName = "fsmathe.mathematik.uni-kl.de";
      User = "jacobsen";
    };
    linda = {
      HostName = "linda.rhrk.uni-kl.de";
      User = "jacobsen";
    };
    lindb = {
      HostName = "lindb.rhrk.uni-kl.de";
      User = "jacobsen";
    };
    skylla = {
      HostName = "skylla.mathematik.uni-kl.de";
      User = "jacobsen";
    };
  }
  // builtins.listToAttrs (
    map
      (name: {
        inherit name;
        value = {
          User = "jacobsen";
          HostName = "${name}.math.rptu.de";
          ProxyJump = "skylla";
        };
      })
      [
        "cipserv01"
        "cipserv02"
        "cipserv03"
        "cipserv04"
        "cipserv05"
      ]
  );
in
{
  packages = [ pkgs.openssh ];

  files.".ssh/config" = {
    generator = helpers.generators.toSSHConfig;
    value = hosts;
  };
}
