{ pkgs, ... }:
{
  packages = [ pkgs.signal-desktop ];

  preservation.preserveAt.state-dir.directories = [ ".config/Signal" ];
}
