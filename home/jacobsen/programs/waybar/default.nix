{
  config,
  lib,
  pkgs,
  ...
}:
{
  stylix.targets.waybar.enable = false;

  programs.waybar = {
    enable = true;
    systemd.enable = true;
    style =
      with config.lib.stylix.colors.withHashtag;
      /* css */ ''
        @define-color base00 ${base00};
        @define-color base01 ${base01};
        @define-color base02 ${base02};
        @define-color base03 ${base03};

        @define-color base04 ${base04};
        @define-color base05 ${base05};
        @define-color base06 ${base06};
        @define-color base07 ${base07};

        @define-color base08 ${base08};
        @define-color base09 ${base09};
        @define-color base0A ${base0A};
        @define-color base0B ${base0B};
        @define-color base0C ${base0C};
        @define-color base0D ${base0D};
        @define-color base0E ${base0E};
        @define-color base0F ${base0F};

        * {
          font-family: "${config.stylix.fonts.monospace.name}";
          font-size: ${toString config.stylix.fonts.sizes.terminal}pt;
        }
      ''
      + (builtins.readFile ./style.css);
    settings = [
      {
        layer = "top";
        exclusive = true;
        passthrough = false;
        position = "bottom";
        spacing = 6;
        fixed-center = true;
        ipc = true;
        margin-top = 0;
        margin-left = 0;
        margin-right = 0;
        modules-left = [
          # group/wayland
          "sway/mode"
          "ext/workspaces"
          "sway/window"
        ];
        modules-center = [
        ];
        modules-right = [
          # group/performance
          "cpu"
          "memory"
          # "temperature"
          "disk"

          "custom/separator#blank_2"

          # group/settings
          "network"
          "bluetooth"
          "backlight"
          "battery"
          "wireplumber"

          "custom/separator#blank_2"

          # group/dbus
          "tray"
          "custom/swaync"
          "idle_inhibitor"

          # group/clock
          "clock"
        ];

        "ext/workspaces" = {
          all-outputs = false;
          disable-click = true;
          disable-scroll = true;
        };
        backlight = {
          interval = 2;
          align = 0;
          rotate = 0;
          format-icons = [
            ""
            ""
            ""
            ""
            ""
            ""
            ""
            ""
            ""
            ""
            ""
            ""
            ""
            ""
            ""
          ];
          format = "{icon}";
          tooltip-format = "backlight {percent}%";
          icon-size = 10;
          on-click = "${lib.getExe pkgs.better-control} --display";
          smooth-scrolling-threshold = 1;
        };
        battery = {
          align = 0;
          rotate = 0;
          full-at = 100;
          design-capacity = false;
          states = {
            good = 95;
            warning = 30;
            critical = 15;
          };
          format = "{icon} {capacity}%";
          format-charging = " {capacity}%";
          format-plugged = "󱘖 {capacity}%";
          #format-full = "{icon} Full";
          format-icons = [
            "󰂎"
            "󰁺"
            "󰁻"
            "󰁼"
            "󰁽"
            "󰁾"
            "󰁿"
            "󰂀"
            "󰂁"
            "󰂂"
            "󰁹"
          ];
          format-time = "{H}h {M}min";
          tooltip = true;
          tooltip-format = "{timeTo} {power}w";
          on-click = "${lib.getExe pkgs.better-control} --battery";
        };
        clock = {
          interval = 60;
          format = "{:%a %d %b %H:%M}";
          # format = "{:%a %d %b %H:%M:%S}";
          tooltip = false;
        };
        "sway/mode" = {
          tooltip = false;
        };
        "sway/window" = {
          format = "{}";
          max-length = 64;
          tooltip = false;
        };
        idle_inhibitor = {
          format = "{icon}";
          format-icons = {
            activated = " ";
            deactivated = " ";
          };
        };
        bluetooth = {
          format = "";
          format-disabled = "󰂳";
          format-connected = "󰂱 {num_connections}";
          tooltip-format = " {device_alias}";
          tooltip-format-connected = "{device_enumerate}";
          tooltip-format-enumerate-connected = " {device_alias} 󰂄{device_battery_percentage}%";
          tooltip = true;
          on-click = "${lib.getExe pkgs.better-control} --bluetooth";
        };
        network = {
          format = "{ifname}";
          format-wifi = "{icon}";
          format-ethernet = "󰌘";
          format-disconnected = "󰌙";
          tooltip-format = "{ipaddr}  {bandwidthUpBytes}  {bandwidthDownBytes}";
          format-linked = "󰈁 {ifname} (No IP)";
          tooltip-format-wifi = "{essid} {icon} {signalStrength}%";
          tooltip-format-ethernet = "{ifname} 󰌘";
          tooltip-format-disconnected = "󰌙 Disconnected";
          max-length = 50;
          format-icons = [
            "󰤯"
            "󰤟"
            "󰤢"
            "󰤥"
            "󰤨"
          ];
          on-click = "${lib.getExe pkgs.better-control} --wifi";
        };
        cpu = {
          format = "󰾆 {usage}%";
          interval = 5;
          on-click = "footclient --title btop sh -c 'btop'";
        };
        disk = {
          interval = 30;
          path = "/persist/";
          format = "󰋊 {percentage_used}%";
          tooltip-format = "{used} used out of {total} on {path} ({percentage_used}%)";
          on-click = "baobab /persist";
        };
        memory = {
          interval = 10;
          format = "󰍛 {percentage}%";
          tooltip = true;
          tooltip-format = "{used:0.1f}G used out of {total:0.1f}G ({percentage}%)";
          on-click = "footclient --title btop sh -c 'btop'";
        };
        temperature = {
          interval = 10;
          tooltip = true;
          hwmon-path = [
            "/sys/class/hwmon/hwmon1/temp1_input"
            "/sys/class/thermal/thermal_zone0/temp"
          ];
          critical-threshold = 82;
          format-critical = "{icon} {temperatureC}°C";
          format = "{icon} {temperatureC}°C";
          format-icons = [ "󰈸" ];
          on-click = "footclient --title btop sh -c 'btop'";
        };
        tray = {
          icon-size = 15;
          spacing = 8;
        };
        wireplumber = {
          format = "{icon} {volume}%";
          format-muted = "";
          on-click = "${lib.getExe pkgs.better-control} --volume";
          format-icons = [
            ""
            ""
            "󰕾"
            " "
          ];
        };
        "custom/swaync" = {
          tooltip = true;
          format = "{icon} {text}";
          format-icons =
            let
              red = config.lib.stylix.colors.withHashtag.base08;
            in
            {
              notification = "<span foreground='${red}'><sup></sup></span>";
              none = "";
              dnd-notification = "<span foreground='${red}'><sup></sup></span>";
              dnd-none = "";
              inhibited-notification = "<span foreground='${red}'><sup></sup></span>";
              inhibited-none = "";
              dnd-inhibited-notification = "<span foreground='${red}'><sup></sup></span>";
              dnd-inhibited-none = "";
            };
          return-type = "json";
          exec-if = "which swaync-client";
          exec = "swaync-client -swb";
          on-click = "sleep 0.1 && swaync-client -t -sw";
          on-click-right = "swaync-client -d -sw";
          escape = true;
        };
        "custom/separator#blank_2" = {
          format = " ";
          interval = "once";
          tooltip = false;
        };
      }
    ];
  };
}
