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
      enable = lib.mkEnableOption "Jay, a tiling wayland compositor";

      package = lib.mkPackageOption pkgs "jay" { };

      realtime-scheduling = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = ''
          Wrap the Jay binary with CAP_SYS_NICE so it can elevate its scheduler to SCHED_RR
          and create high-priority Vulkan queues, improving responsiveness under load.

          For security, Jay only elevates to SCHED_RR if every config.so in the config directory
          is "privileged" (owned by root:root, not group/world-writable). config.so packages built
          by Nix and installed via the store satisfy this automatically.
        '';
      };

      xwayland.enable = lib.mkEnableOption "XWayland" // {
        default = true;
      };

      extraPackages = lib.mkOption {
        type = with lib.types; listOf package;
        example = lib.literalExpression ''
          with pkgs; [ alacritty bemenu mako wl-tray-bridge ];
        '';
        description = ''
          Extra packages to be installed system wide.
        '';
      };
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages =
      (lib.optional (!cfg.realtime-scheduling) cfg.package) ++ cfg.extraPackages;

    programs = {
      dconf.enable = lib.mkDefault true;
      xwayland.enable = lib.mkIf cfg.xwayland.enable (lib.mkDefault true);
    };

    security = {
      polkit.enable = true;
      pam.services.swaylock = { };

      wrappers = lib.mkIf cfg.realtime-scheduling {
        jay = {
          owner = "root";
          group = "root";
          permissions = "a+rx";
          source = lib.getExe cfg.package;
          capabilities = "cap_sys_nice+p";
        };
      };
    };

    xdg.portal = {
      enable = lib.mkDefault true;
      configPackages = lib.mkDefault [ cfg.package ];
      extraPortals = [ pkgs.xdg-desktop-portal-gtk ];
    };

    services.displayManager.sessionPackages = [ cfg.package ];
  };
}
