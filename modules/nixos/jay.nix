{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.programs.jay;
in
{
  options = {
    programs.jay = {
      enable = lib.mkEnableOption "jay, a tiling wayland compositor";

      package = lib.mkPackageOption pkgs "jay" { };

      xwayland.enable = lib.mkEnableOption "XWayland" // {
        default = true;
      };

      extraPackages = lib.mkOption {
        type = with lib.types; listOf package;
        default = with pkgs; [
          foot
          fuzzel
          swaylock
        ];
        defaultText = lib.literalExpression ''
          with pkgs; [ foot fuzzel swaylock ];
        '';
        example = lib.literalExpression ''
          with pkgs; [ brightnessctl wl-clipboard ]
        '';
        description = ''
          Extra packages to be installed system wide.
        '';
      };
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = [ cfg.package ] ++ cfg.extraPackages;

    programs = {
      dconf.enable = lib.mkDefault true;
      xwayland.enable = lib.mkIf cfg.xwayland.enable (lib.mkDefault true);
    };

    security = {
      polkit.enable = true;
      pam.services.swaylock = { };
    };

    xdg.portal = {
      enable = lib.mkDefault true;
      configPackages = lib.mkDefault [ cfg.package ];
      extraPortals = [ pkgs.xdg-desktop-portal-gtk ];
    };

    # make jay session available to uwsm
    programs.uwsm.waylandCompositors = {
      jay = {
        prettyName = "Jay";
        comment = "Jay compositor managed by UWSM";
        binPath = "/run/current-system/sw/bin/jay";
        extraArgs = [ "run" ];
      };
    };

    # make jay session available to display managers
    services.displayManager.sessionPackages = [ cfg.package ];
  };
}
