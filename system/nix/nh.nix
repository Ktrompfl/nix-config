{ lib, ... }:
{
  programs.nh = {
    enable = true;
    flake = "/persist/nixos/";

    # This is the store's garbage collection; `nix.gc` is deliberately left
    # alone so that only one of the two ever deletes anything.
    clean = {
      enable = true;
      extraArgs = "--keep 5 --keep-since 30d";
      dates = "weekly";
    };
  };

  # Only on mains power, and never catching up on a run that was missed while
  # the machine was off; this is housekeeping, not worth fighting the user for
  # the disk over.
  systemd.timers.nh-clean = {
    unitConfig.ConditionACPower = true;

    timerConfig = {
      Persistent = lib.mkForce false;
      RandomizedDelaySec = lib.mkForce "2h";
    };
  };

  # The scheduling priorities belong on the service. systemd ignores
  # `IOSchedulingClass` and `CPUSchedulingPolicy` in a timer's `[Unit]`
  # section, and says so in the journal on every boot.
  systemd.services.nh-clean.serviceConfig = {
    IOSchedulingClass = "idle";
    CPUSchedulingPolicy = "idle";
  };
}
