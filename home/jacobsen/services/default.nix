{ lib, ... }:
{
  imports = [
    ./awww.nix
    ./cliphist.nix
    ./foot.nix
    ./swayidle.nix
    ./swaync
    ./wayland-pipewire-idle-inhibit.nix
    ./wl-clip-persist.nix
    ./wl-tray-bridge.nix
  ];

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
