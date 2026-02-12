{ pkgs, ... }:
{
  programs.steam = {
    gamescopeSession.enable = true;
    remotePlay.openFirewall = true; # open ports in the firewall for steam remote play
    dedicatedServer.openFirewall = true; # open ports in the firewall for source dedicated server
    extraCompatPackages = [ pkgs.proton-ge-bin ]; # add proton ge

    # add missing dependencies for runners and games
    package = pkgs.steam.override {
      extraPkgs =
        pkgs: with pkgs; [
          # fix undefined symbols in X11 session, otherwise gamescope fails in steam
          xorg.libXcursor
          xorg.libXi
          xorg.libXinerama
          xorg.libXScrnSaver
          libpng
          libpulseaudio
          libvorbis
          stdenv.cc.cc.lib
          libkrb5
          keyutils
        ];
    };
  };
}
