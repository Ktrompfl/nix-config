{ config, lib, ... }:
let
  cfg = config.services.pipewire;
in
lib.mkIf cfg.enable {
  # rtkit allows pipewire to use the realtime scheduler for increased performance
  security.rtkit.enable = lib.mkDefault true;

  services.pipewire = {
    alsa.enable = lib.mkDefault true;
    alsa.support32Bit = lib.mkDefault true;
    pulse.enable = lib.mkDefault true;
    wireplumber.enable = lib.mkDefault true;
  };

  users.users.jacobsen.extraGroups = [
    "audio"
    "video"
  ];

  preservation.preserveAt.state-dir.directories = [ "/var/lib/pipewire" ];
}
