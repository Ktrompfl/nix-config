{ lib, ... }:
{
  # Hard-links identical files in the store. `auto-optimise-store` would do the
  # same on every write, which costs something on every single build; once a
  # week off-peak is enough. Deleting store paths is nh's job, see ./nh.nix.
  nix.optimise = {
    automatic = true;
    dates = [ "weekly" ];
  };

  # Only on mains power, and never catching up on a run that was missed while
  # the machine was off; this is housekeeping, not worth fighting the user for
  # the disk over.
  systemd.timers.nix-optimise = {
    unitConfig.ConditionACPower = true;

    timerConfig = {
      Persistent = lib.mkForce false;
      RandomizedDelaySec = lib.mkForce "2h";
    };
  };

  # The scheduling priorities belong on the service. systemd ignores
  # `IOSchedulingClass` and `CPUSchedulingPolicy` in a timer's `[Unit]`
  # section, and says so in the journal on every boot.
  systemd.services.nix-optimise.serviceConfig = {
    IOSchedulingClass = "idle";
    CPUSchedulingPolicy = "idle";
  };
}
