{ config, ... }:
let
  colors = config.lib.stylix.colors.withHashtag;

  netDownIcon = "󰌙";
  batteryFullIcon = "󱘖";
  bluetoothUnavailableIcon = "󰂳";
  notificationBadgeIcon = "";
  idleInhibitorOffIcon = "";
in
{
  programs.i3status-rust = {
    enable = true;

    bars.jay = {
      settings = {
        theme = {
          theme = "native";
          overrides = {
            info_fg = colors.base0A;
            warning_fg = colors.base09;
            critical_fg = colors.base08;
          };
        };

        icons = {
          icons = "material-nf";
          overrides = {
            cpu = "󰾆";
            memory_mem = "󰍛";
            disk_drive = "󰋊";
            net_wireless = [
              "󰤯"
              "󰤟"
              "󰤢"
              "󰤥"
              "󰤨"
            ];
            net_wired = "󰌘";
            backlight = [
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
            bat = [
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
            bat_charging = "";
            volume = [
              ""
              ""
              "󰕾"
            ];
            volume_muted = "";
            bell = "";
            "bell-slash" = "";
            bluetooth = "󰂱";
          };
        };
      };

      blocks = [
        {
          block = "custom_dbus";
          path = "/mode";
          format = {
            full = " {$text.pango-str()|NORMAL} ";
            short = "";
          };
        }

        {
          block = "cpu";
          interval = 2;
          format = " $icon $utilization ";
          info_cpu = 30;
          warning_cpu = 60;
          critical_cpu = 90;
        }

        {
          block = "memory";
          interval = 5;
          format = " $icon $mem_used_percents ";
          warning_mem = 80;
          critical_mem = 95;
        }

        {
          block = "disk_space";
          path = "/persist";
          interval = 30;
          info_type = "used";
          format = " $icon $percentage ";
          warning = 80;
          alert = 90;
        }

        {
          block = "net";
          interval = 5;
          format = " $icon ";
          inactive_format = " ${netDownIcon} ";
          missing_format = " ${netDownIcon} ";
        }

        {
          block = "bluetooth";
          # Unlike the rest of these blocks, bluetooth tracks one specific
          # device rather than reporting adapter-wide state, and `mac` has no
          # default - deserializing the whole config fails (taking every other
          # block down with it) if it is missing entirely, but not if it is
          # merely wrong, so this is a placeholder: find the real address with
          # `bluetoothctl devices` and replace it.
          mac = "00:00:00:00:00:00";
          format = " $icon{ $percentage|} ";
          disconnected_format = "{$available| ${bluetoothUnavailableIcon} }";
          battery_state."0..100" = "idle";
        }

        {
          block = "backlight";
          format = " $icon ";
          missing_format = "";
        }

        {
          block = "battery";
          driver = "sysfs";
          interval = 10;
          format = " $icon $percentage ";
          charging_format = " $icon $percentage ";
          full_format = " ${batteryFullIcon} $percentage ";
          not_charging_format = " ${batteryFullIcon} $percentage ";
          empty_format = " $icon $percentage ";
          missing_format = "";
          info = 0;
          good = 100;
          warning = 30;
          critical = 15;
        }

        {
          block = "sound";
          driver = "pipewire";
          device_kind = "sink";
          show_volume_when_muted = false;
          format = " $icon{ $volume|} ";
        }

        {
          block = "notify";
          driver = "swaync";
          format = " $icon{<span foreground='${colors.base08}'><sup>${notificationBadgeIcon}</sup></span> $notification_count.eng(w:1)|} ";
          theme_overrides.info_fg = "none";
        }

        {
          block = "custom_dbus";
          path = "/idle_inhibitor";
          format = {
            full = " {$text.pango-str()|${idleInhibitorOffIcon}} ";
            short = "";
          };
        }

        {
          block = "time";
          interval = 60;
          format = " $timestamp.datetime(f:'%a %d %b %H:%M') ";
        }
      ];
    };
  };
}
