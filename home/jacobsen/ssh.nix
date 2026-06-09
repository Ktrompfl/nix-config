{
  programs.ssh = {
    enable = true;
    settings = {
      "*" = {
        ForwardAgent = false;
        AddKeysToAgent = "no";
        Compression = false;
        ServerAliveInterval = 15;
        ServerAliveCountMax = 3;
        HashKnownHosts = false;
        UserKnownHostsFile = "~/.ssh/known_hosts";
        ControlMaster = "auto";
        ControlPath = "~/.ssh/master-%r@%n:%p";
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
  };

  # persist ssh keys / known hosts
  preservation.preserveAt.data-dir.directories = [
    {
      directory = ".ssh";
      mode = "0700";
    }
  ];
}
