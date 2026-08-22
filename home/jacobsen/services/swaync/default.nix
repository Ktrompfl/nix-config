{
  config,
  graphicalService,
  lib,
  pkgs,
  ...
}:
{
  packages = [ pkgs.swaynotificationcenter ];

  # notifications fail silently if this is killed
  systemd.services.swaync = graphicalService "session" {
    description = "Swaync notification daemon";
    serviceConfig.ExecStart = lib.getExe' pkgs.swaynotificationcenter "swaync";
  };

  xdg.config.files = {
    "swaync/style.css".text =
      with config.theme.colors.withHashtag;
      ''
        * {
            font-family: "${config.theme.fonts.sansSerif.name}";
            font-size: ${toString config.theme.fonts.sizes.popups}pt;
        }

        @define-color base00 ${base00}; @define-color base01 ${base01};
        @define-color base02 ${base02}; @define-color base03 ${base03};
        @define-color base04 ${base04}; @define-color base05 ${base05};
        @define-color base06 ${base06}; @define-color base07 ${base07};

        @define-color base08 ${base08}; @define-color base09 ${base09};
        @define-color base0A ${base0A}; @define-color base0B ${base0B};
        @define-color base0C ${base0C}; @define-color base0D ${base0D};
        @define-color base0E ${base0E}; @define-color base0F ${base0F};
      ''
      + builtins.readFile ./style.css;

    "swaync/config.json" = {
      generator = (pkgs.formats.json { }).generate "swaync-config.json";
      value = {
        positionX = "right";
        positionY = "top";
        cssPriority = "user";
        control-center-width = 420;
        notification-window-width = 420;
        notification-icon-size = 48;
        notification-body-image-height = 160;
        notification-body-image-width = 200;
        timeout = 4;
        timeout-low = 2;
        timeout-critical = 6;
        fit-to-screen = true;
        keyboard-shortcuts = true;
        image-visibility = "when-available";
        transition-time = 100;
        hide-on-clear = false;
        hide-on-action = false;
        script-fail-notify = true;
        notification-visibility = {
          example-name = {
            state = "muted";
            urgency = "Low";
            app-name = "Firefox";
          };
        };
        widgets = [
          "mpris"
          "title"
          "dnd"
          "notifications"
        ];
        widget-config = {
          title = {
            text = "Notifications";
            clear-all-button = true;
            button-text = "  ";
          };
          dnd = {
            text = "Do not disturb";
          };
          mpris = {
            image-size = 96;
            image-radius = 12;
          };
          volume = {
            label = "󰕾";
            show-per-app = true;
          };
        };
      };
    };
  };
}
