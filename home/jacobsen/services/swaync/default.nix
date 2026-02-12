{
  config,
  ...
}:
{
  services.swaync = {
    enable = true;
    style =
      with config.lib.stylix.colors.withHashtag;
      ''
        @define-color base00 ${base00}; @define-color base01 ${base01};
        @define-color base02 ${base02}; @define-color base03 ${base03};
        @define-color base04 ${base04}; @define-color base05 ${base05};
        @define-color base06 ${base06}; @define-color base07 ${base07};

        @define-color base08 ${base08}; @define-color base09 ${base09};
        @define-color base0A ${base0A}; @define-color base0B ${base0B};
        @define-color base0C ${base0C}; @define-color base0D ${base0D};
        @define-color base0E ${base0E}; @define-color base0F ${base0F};
      ''
      + (builtins.readFile ./style.css);
    settings = {
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
}
