{ pkgs, ... }:
{
  packages = [ (pkgs.chromium.override { enableWideVine = true; }) ];

  preservation.preserveAt.state-dir.directories = [ ".config/chromium" ];
}
