{ pkgs, ... }:
{
  packages = [ pkgs.libreoffice ];

  preservation.preserveAt.state-dir.directories = [ ".config/libreoffice" ];
}
