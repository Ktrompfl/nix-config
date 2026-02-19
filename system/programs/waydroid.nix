{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.virtualisation.waydroid;
in
lib.mkIf cfg.enable {
  # install waydroid-script to install libhoudini / libndk
  environment.systemPackages = [ pkgs.nur.repos.ataraxiasjel.waydroid-script ];

  preservation.preserveAt.state-dir.directories = [ "/var/lib/waydroid" ];

  # enable ip forwarding
  nix-mineral.settings.network.ip-forwarding = true;
}
