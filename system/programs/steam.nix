{ pkgs, ... }:
{
  programs.steam = {
    remotePlay.openFirewall = true; # open ports in the firewall for steam remote play
    dedicatedServer.openFirewall = true; # open ports in the firewall for source dedicated server
    extraCompatPackages = [ pkgs.proton-ge-bin ]; # add proton ge
  };
}
