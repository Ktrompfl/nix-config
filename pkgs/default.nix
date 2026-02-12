# Custom packages, that can be defined similarly to ones from nixpkgs
# You can build them using 'nix build .#example'
pkgs: {
  glfw-waywall = pkgs.callPackage ./glfw-waywall.nix { };
  jay = pkgs.callPackage ./jay.nix { };
  ninjabrainbot = pkgs.callPackage ./ninjabrainbot.nix { };
}
