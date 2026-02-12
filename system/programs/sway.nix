{ config, lib, ... }:
let
  cfg = config.programs.sway;
in
lib.mkIf cfg.enable {
  programs.uwsm = {
    enable = lib.mkDefault true;
    waylandCompositors = {
      sway = {
        prettyName = "Sway";
        comment = "Sway compositor managed by UWSM";
        binPath = "/run/current-system/sw/bin/sway";
      };
    };
  };
}
