{
  programs.direnv = {
    enable = true;
    nix-direnv.enable = true;
  };

  preservation.preserveAt.state-dir.directories = [
    ".local/share/direnv"
  ];
}
