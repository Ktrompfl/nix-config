{
  config,
  lib,
  pkgs,
  ...
}:
let
  lock = "${lib.getExe pkgs.swaylock} --daemonize";
in
{
  packages = [ pkgs.swaylock ];

  xdg.config.files."swaylock/config" = {
    generator = lib.generators.toKeyValueLines { flags = true; };

    value = with config.theme.colors.withoutHashtag; {
      color = base00;
      scaling = "fill";
      separator-color = "00000000";

      inside-color = base00;
      inside-clear-color = base00;
      inside-caps-lock-color = base00;
      inside-ver-color = base00;
      inside-wrong-color = base00;

      ring-color = base01;
      ring-clear-color = base08;
      ring-caps-lock-color = base01;
      ring-ver-color = base0B;
      ring-wrong-color = base08;

      key-hl-color = base0B;

      text-color = base05;
      text-clear-color = base05;
      text-caps-lock-color = base05;
      text-ver-color = base05;
      text-wrong-color = base05;

      layout-bg-color = base00;
      layout-border-color = base01;
      layout-text-color = base05;

      ignore-empty-password = true;
      line-uses-inside = true;
    };
  };

  wayland.services = {
    swayidle = {
      description = "Idle manager for Wayland";
      sandbox = false;
      serviceConfig = {
        Environment = [ "PATH=${lib.makeBinPath [ pkgs.bash ]}" ];
        ExecStart = lib.concatStringsSep " " [
          (lib.getExe pkgs.swayidle)
          "-w"
          "lock '${lock}'"
          "before-sleep '${lock}'"
        ];
      };
      slice = "session";
    };

    wayland-pipewire-idle-inhibit =
      let
        settings = (pkgs.formats.toml { }).generate "wayland-pipewire-idle-inhibit.toml" {
          verbosity = "INFO";
          media_minimum_duration = 5;
        };
      in
      {
        description = "Inhibit idle when audio is playing";
        serviceConfig.ExecStart = "${lib.getExe pkgs.wayland-pipewire-idle-inhibit} --config ${settings}";
      };
  };
}
