{
  programs.nh = {
    enable = true;
    # weekly cleanup
    clean = {
      enable = true;
      extraArgs = "--keep 5 --keep-since 30d";
      dates = "weekly";
    };
    flake = "/persist/nixos/";
  };
}
