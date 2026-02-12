{ pkgs, ... }:
{
  programs.chromium = {
    # chromium as fallback browser
    enable = true;
    package = pkgs.chromium.override { enableWideVine = true; };
  };

  # electron apps store their state inseparably in .config/
  preservation.preserveAt.state-dir.directories = [
    ".config/chromium"
  ];
}
