{
  config,
  pkgs,
  ...
}:
{
  packages = [ pkgs.i3status-rust ];

  xdg.config.files."i3status-rust/config-jay.toml" = {
    generator = (pkgs.formats.toml { }).generate "i3status-rust-jay.toml";
    value = with config.theme.colors.withHashtag; {

      theme = {
        theme = "native";
        overrides = {
          info_fg = base0A;
          warning_fg = base09;
          critical_fg = base08;
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

      block = [
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
          inactive_format = " 󰌙 ";
          missing_format = " 󰌙 ";
        }

        {
          block = "bluetooth";
          mac = "00:00:00:00:00:00";
          format = " $icon{ $percentage|} ";
          disconnected_format = "{$available| 󰂳 }";
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
          full_format = " 󱘖 $percentage ";
          not_charging_format = " 󱘖 $percentage ";
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
          format = " $icon{<span foreground='${base08}'><sup></sup></span> $notification_count.eng(w:1)|} ";
          theme_overrides.info_fg = "none";
        }

        {
          block = "custom_dbus";
          path = "/idle_inhibitor";
          format = {
            full = " {$text.pango-str()|} ";
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
