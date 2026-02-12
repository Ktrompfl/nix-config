{
  # nix-ld allows to run unpatched dynamic binaries on NixOS, see https://github.com/nix-community/nix-ld
  # this is rather impure, but currently the easiest way to use julia packages with binary artifacts, see https://discourse.julialang.org/t/best-way-to-install-and-use-julia-on-nix-nixos/109948/15
  programs.nix-ld.enable = true;
}
