{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.programs.via;
in
{
  options = {
    programs.via = {
      enable = lib.mkEnableOption "via for configuring keyboards via qmk";
      package = lib.mkPackageOption pkgs "via" { };
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ];

    services.udev.packages = [ cfg.package ];

    hardware.keyboard.qmk.enable = true;
  };
}
