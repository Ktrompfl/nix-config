{ pkgs, ... }:
{
  home.packages = [ pkgs.signal-desktop ];

  # electron apps store their state inseparably in .config/
  preservation.preserveAt.state-dir.directories = [
    ".config/Signal"
  ];
}
