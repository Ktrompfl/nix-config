{ pkgs, ... }:
{
  packages = [ pkgs.seafile-client ];

  preservation.preserveAt = {
    state-dir.directories = [ ".ccnet" ];
    data-dir.directories = [ "Seafile" ];
  };
}
