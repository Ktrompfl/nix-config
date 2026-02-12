{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.programs.piper;
in
{
  options = {
    programs.piper = {
      enable = lib.mkEnableOption "piper for configuring gaming mice via ratbagd";
      package = lib.mkPackageOption pkgs "piper" { };
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];

    services.ratbagd.enable = true;
  };
}
