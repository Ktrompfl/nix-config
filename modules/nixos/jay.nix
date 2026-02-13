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

      realtime-scheduling = lib.mkOption {
        type = lib.types.bool;
        default = true;
        description = ''
          Add CAP_SYS_NICE capabilities to the Jay binary.

          If CAP_SYS_NICE is available, Jay will, by default, elevate its scheduler to SCHED_RR
          and create Vulkan queues with the highest available priority.
          This can improve responsiveness if the CPU or GPU are under high load.

          If Jay is started with the environment variable JAY_NO_REALTIME=1 or a config.so exists,
          then Jay will not elevate its scheduler but will still create elevated Vulkan queues.

          Jay will drop all capabilities almost immediately after being started. Before that,
          it will spawn a dedicated thread that retains the CAP_SYS_NICE capability to create
          elevated Vulkan queues later.

          If Jay has elevated its scheduler to SCHED_RR, then it will refuse to load config.so configurations.
          Otherwise unprivileged applications would be able to run arbitrary code with SCHED_RR by
          crafting a dedicated config.so. This behavior can be overridden by compiling Jay
          with JAY_ALLOW_REALTIME_CONFIG_SO=1.
        '';
      };

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
          source = "${lib.getExe cfg.package}";
          capabilities = "cap_sys_nice+p";
        };
      };
    };

    xdg.portal = {
      enable = lib.mkDefault true;
      configPackages = lib.mkDefault [ cfg.package ];
      extraPortals = [ pkgs.xdg-desktop-portal-gtk ];
    };

    # make jay session available to display managers and uwsm
    services.displayManager.sessionPackages = [ cfg.package ];
  };
}
