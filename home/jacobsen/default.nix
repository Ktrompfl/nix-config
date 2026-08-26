{ pkgs, ... }: {
  imports = [
    ./programs/claude
    ./programs/neovim

    ./programs/btop.nix
    ./programs/fastfetch.nix
    ./programs/gh.nix
    ./programs/git.nix
    ./programs/julia.nix
    ./programs/latex.nix
    ./programs/matplotlib.nix
    ./programs/python.nix
    ./programs/rust.nix
    ./programs/shell.nix
    ./programs/ssh.nix

    ./environment.nix
  ];

  packages = with pkgs; [
    # nix tools
    manix

    # languages
    php
    typst
  ];
}
