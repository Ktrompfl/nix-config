{ lib, ... }:
{
  imports = [
    ./awww.nix
    ./cliphist.nix
    ./foot.nix
    ./swayidle.nix
    ./swaync.nix
    ./wayland-pipewire-idle-inhibit.nix
    ./wl-clip-persist.nix
    ./wl-tray-bridge.nix
  ];

  # Bound to the graphical session; the slice is what each unit picks for
  # itself, see ../../system/oomd.nix for what the three of them mean.
  _module.args.graphicalService =
    slice: unit:
    lib.recursiveUpdate {
      partOf = [ "graphical-session.target" ];
      after = [ "graphical-session.target" ];
      wantedBy = [ "graphical-session.target" ];

      serviceConfig = {
        Type = "simple";
        Restart = "on-failure";
        Slice = "${slice}-graphical.slice";
      };
    } unit;
}
