{ config, lib, ... }:
let
  cfg = config.programs.corectrl;
in
{
  users.users.jacobsen.extraGroups = lib.mkIf cfg.enable [ "corectrl" ];
}
