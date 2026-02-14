# Custom packages, that can be defined similarly to ones from nixpkgs
# You can build them using 'nix build .#example'
pkgs: {
  jay = pkgs.callPackage ./jay.nix { };
  ninjabrainbot = pkgs.callPackage ./ninjabrainbot.nix { };
}
