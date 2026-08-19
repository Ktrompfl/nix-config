{ lib, ... }: {
  programs.nh = {
    enable = true;
    # weekly cleanup
    clean = {
      enable = true;
      extraArgs = "--keep 5 --keep-since 30d";
      dates = "weekly";
    };
    flake = "/persist/nixos/";
  };

  systemd.timers.nh-clean = {
    unitConfig = {
      ConditionACPower = true;
      IOSchedulingClass = "idle";
      CPUSchedulingPolicy = "idle";
    };
    timerConfig = {
      Persistent = lib.mkForce false;
      RandomizedDelaySec = lib.mkForce "2h";
    };
  };
}
