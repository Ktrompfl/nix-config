{
  config,
  inputs,
  pkgs,
  ...
}:
{
  imports = [
    inputs.wayland-pipewire-idle-inhibit.homeModules.default
  ];

  services.wayland-pipewire-idle-inhibit = {
    enable = true;
    # use package from nixpkgs to avoid building from source
    package = pkgs.wayland-pipewire-idle-inhibit;
    systemdTarget = config.wayland.systemd.target;
    settings = {
      verbosity = "INFO";
      media_minimum_duration = 5;
    };
  };
}
