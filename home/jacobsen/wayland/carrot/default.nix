{
  config,
  inputs,
  lib,
  pkgs,
  ...
}:
let
  settings = {
    input = {
      # FIXME: only one input can be configured
      keyboard.xkb = {
        layout = "us";
        variant = "altgr-intl";
        options = "caps:escape";
      };

      touchpad = {
        accel_profile = "flat";
        accel_speed = 0.0;
        natural_scroll = false;
      };
      mouse = {
        accel_profile = "flat";
        accel_speed = 0.0;
        natural_scroll = false;
      };

      mod_key = "super";
    };

    layout = {
      mode = "dwindle";
      gaps_in = 0;
      gaps_out = 0;
      border = {
        width = 1;
        active_color = config.theme.colors.hex "accent";
        inactive_color = config.theme.colors.hex "muted";
      };
    };

    animations.off = true;
    prefer_no_csd = true;

    outputs = {
      "eDP-1" = {
        mode = "1920x1080@60";
        position = {
          x = 0;
          y = 0;
        };
      };
      "EPSON PJ" = {
        mode = "1920x1080@60";
        position = {
          x = 0;
          y = 1080;
        };
      };
      "VG270U P" = {
        mode = "2560x1440@144";
        position = {
          x = 0;
          y = 240;
        };
        scale = 1.0;
        vrr = "on-demand";
      };
      "BenQ GL2480" = {
        mode = "1920x1080@60";
        position = {
          x = 2560;
          y = 0;
        };
        scale = 1.0;
        # FIXME: output needs rotation/transform
      };
    };

    environment = import ../env.nix config;

    binds = import ./binds.nix { inherit lib pkgs; };
  };
in
{
  packages = [ inputs.carrot.packages.${pkgs.stdenv.hostPlatform.system}.carrot ];

  xdg.config.files."carrot/carrot.lua".text = ''
    carrot = ${lib.generators.toLua { } settings}
  '';
}
