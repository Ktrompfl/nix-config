{ pkgs, ... }:
{
  programs.steam = {
    remotePlay.openFirewall = true; # open ports in the firewall for steam remote play
    dedicatedServer.openFirewall = true; # open ports in the firewall for source dedicated server
    extraCompatPackages = [ pkgs.proton-ge-bin ]; # add proton ge
    package = pkgs.steam.override {
      extraEnv = {
        MANGOHUD = "1";
        # GAMEMODERUN = "1";
        PROTON_ENABLE_WAYLAND = "1";
        PROTON_ENABLE_HDR = "1";
        PROTON_FSR4_RDNA3_UPGRADE = "1";
      };
    };
  };
}
