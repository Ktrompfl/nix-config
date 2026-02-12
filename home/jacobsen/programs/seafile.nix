{ pkgs, ... }:
{
  home.packages = [ pkgs.seafile-client ];

  preservation.preserveAt.data-dir.directories = [
    "Seafile"
  ];
}
