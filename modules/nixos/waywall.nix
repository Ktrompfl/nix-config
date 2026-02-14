{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.programs.waywall;
in
{
  options = {
    programs.waywall = {
      enable = lib.mkEnableOption ''
        Wayland compositor for Minecraft speedrunning

        Note: To run Minecraft inside Waywall, a patched version of GLFW is required.
        This version is available as 'pkgs.glfw3-minecraft' and included in 'pkgs.prismlauncher'.

        To use waywall inside Prism Launcher, go to the settings of an instance and:
        - Select a Java executable with version at least 21 and enable 'Skip Java compatibility check'
            for older Minecraft versions under 'Java'.
        - Enable 'Native Libraries' and 'Use system installation of GLFW' under 'Tweaks'.
        - Set 'waywall wrap --' as 'Wrapper Command' under 'Custom Commands'.
      '';

      package = lib.mkPackageOption pkgs "waywall" { };

      realtime-scheduling = lib.mkOption {
        type = lib.types.bool;
        default = false;
        description = ''
          Add CAP_SYS_NICE capabilities to the Waywall binary.

          If CAP_SYS_NICE is available, Waywall will, by default, elevate its scheduler to SCHED_RR.
        '';
      };
    };
  };

  config = lib.mkIf cfg.enable {
    environment.systemPackages = (lib.optional (!cfg.realtime-scheduling) cfg.package);

    security = {
      wrappers = lib.mkIf cfg.realtime-scheduling {
        waywall = {
          owner = "root";
          group = "root";
          permissions = "a+rx";
          source = "${lib.getExe cfg.package}";
          capabilities = "cap_sys_nice+p";
        };
      };
    };
  };
}
