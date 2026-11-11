{
  config,
  helpers,
  pkgs,
  ...
}:
let
  directory = "${config.storage.data}/.local/share/ssh";
  identities = [ "id_ed25519" ]; # must exist

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
