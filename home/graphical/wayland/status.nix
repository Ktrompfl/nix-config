{
  config,
  lib,
  osConfig,
  pkgs,
  ...
}:
{
  packages = [ pkgs.i3status-rust ];

  xdg.config.files."i3status-rust/config.toml" = {
    generator = (pkgs.formats.toml { }).generate "i3status-rust.toml";
    value =
      with config.theme.colors.withHashtag;
      let
        glyph = text: "<span font_family='${config.theme.fonts.symbols.name}' fgalpha='60%'>${text}</span>";
        alert =
          text: "<span font_family='${config.theme.fonts.symbols.name}' foreground='${error}'>${text}</span>";
        icon = glyph "$icon";
        figureSpace = builtins.fromJSON ''" "'';
        number =
          args: name:
          "<span font_features='tnum'>$" + "${name}.eng(${args}, pad_with:'${figureSpace}')</span>";
        num = number "w:2";
        speed = number "w:3, prefix:K, hide_unit:true";
        gap = "  ";
        divider = "  <span foreground='${base03}'>│</span>  ";

        hasRptuVpn = osConfig.systemd.network.netdevs ? "30-wg-rptu";

        disks = [
          {
            label = "<span fgalpha='60%'>/</span>";
            path = "/";
          }
          {
            label = glyph "󰋊";
            path = "/persist";
          }
        ]
        ++ lib.optional (osConfig.fileSystems ? "/extra") {
          label = glyph "󰆼";
          path = "/extra";
        };

        failedUnits = pkgs.writeShellScript "failed-units" ''
          system=$(systemctl --failed --no-legend --plain | wc -l)
          user=$(systemctl --user --failed --no-legend --plain | wc -l)
          if [ $((system + user)) -gt 0 ]; then echo $((system + user)); fi
        '';
      in
      {

        theme = {
          theme = "native";
          overrides = {
            good_fg = success;
            info_fg = info;
            warning_fg = warning;
            critical_fg = error;
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
              full = "<span foreground='${base0A}' weight='bold'>$text.pango-str()</span>${gap}";
              short = "";
            };
          }

          {
            block = "cpu";
            interval = 2;
            format = "${icon} ${num "utilization"}";
            info_cpu = 30;
            warning_cpu = 60;
            critical_cpu = 90;
          }

          {
            block = "temperature";
            chip = "k10temp-*";
            inputs = [ "Tctl" ];
            interval = 5;
            format = "${gap}${icon} ${num "max"}C";
            idle = 60;
            info = 75;
            warning = 90;
          }

          {
            block = "memory";
            interval = 5;
            format = "${gap}${icon} ${num "mem_used_percents"}";
            warning_mem = 80;
            critical_mem = 95;
          }

          {
            block = "amd_gpu";
            interval = 2;
            format = "${divider}${icon} ${num "utilization"}";
          }

          {
            block = "temperature";
            chip = "amdgpu-*";
            inputs = [ "edge" ];
            interval = 5;
            format = "${gap}${icon} ${num "max"}C";
            idle = 60;
            info = 75;
            warning = 90;
          }

          {
            block = "amd_gpu";
            interval = 5;
            format = "${gap}${glyph "󰢮"} ${num "vram_used_percents"}";
          }

        ]
        ++ lib.imap0 (i: disk: {
          block = "disk_space";
          inherit (disk) path;
          interval = 30;
          info_type = "used";
          format = "${if i == 0 then divider else gap}${disk.label} ${num "percentage"}";
          warning = 80;
          alert = 90;
        }) disks
        ++ [
          {
            block = "net";
            interval = 5;
            format = "${divider}${icon}  ${glyph "^icon_net_down"} ${speed "speed_down"}  ${glyph "^icon_net_up"} ${speed "speed_up"}";
            inactive_format = "${divider}${glyph "󰌙"}";
            missing_format = "${divider}${glyph "󰌙"}";
          }
        ]
        ++ lib.optional hasRptuVpn {
          block = "net";
          device = "wg-rptu";
          interval = 5;
          format = "${gap}${glyph "󰦝"}";
          inactive_format = "";
          missing_format = "";
        }
        ++ [
          {
            block = "sound";
            driver = "pipewire";
            device_kind = "sink";
            show_volume_when_muted = false;
            format = "${divider}${icon}{ ${num "volume"}|}";
          }

          {
            block = "backlight";
            format = "${gap}${icon}";
            missing_format = "";
          }

          {
            block = "battery";
            driver = "sysfs";
            interval = 10;
            format = "${gap}${icon} ${num "percentage"}";
            charging_format = "${gap}${icon} ${num "percentage"}";
            full_format = "${gap}${glyph "󱘖"} ${num "percentage"}";
            not_charging_format = "${gap}${glyph "󱘖"} ${num "percentage"}";
            empty_format = "${gap}${icon} ${num "percentage"}";
            missing_format = "";
            info = 0;
            good = 100;
            warning = 30;
            critical = 15;
          }

          {
            block = "time";
            interval = 60;
            format = "${divider}<span foreground='${base04}'>$timestamp.datetime(f:'%a %d %b')</span>  $timestamp.datetime(f:'%H:%M')";
          }

          {
            block = "weather";
            # Kaiserslautern
            service = {
              name = "metno";
              coordinates = [
                "49.44"
                "7.77"
              ];
            };
            interval = 900;
            format = "${gap}${icon} ${num "temp"}";
          }

          {
            block = "notify";
            driver = "swaync";
            format = "${divider}${icon}{<span foreground='${base08}'><sup></sup></span> ${num "notification_count"}|}";
            theme_overrides.info_fg = "none";
          }

          {
            block = "custom";
            command = "${failedUnits}";
            interval = 60;
            hide_when_empty = true;
            format = "${gap}${alert "󰀦"} <span foreground='${base08}'>$text</span>";
          }

          {
            block = "custom";
            command = "systemctl --user is-active -q idle.service && echo 󰾪 || echo 󰅶";
            interval = 5;
            # trailing gap: the tray follows
            format = "${gap}${glyph "$text"}${gap}";
          }
        ];
      };
  };

  xdg.config.files."i3bar-wl/config.toml" = {
    generator = (pkgs.formats.toml { }).generate "i3bar-wl.toml";
    value = {
      command = [
        (lib.getExe pkgs.i3status-rust)
        "config.toml"
      ];
      env.I3RS_DBUS_NAME = "bar";
      font = "${config.theme.fonts.sansSerif.name} ${toString config.theme.fonts.sizes.desktop}";
      height = 18;
      position = "bottom";
      colors = with config.theme.colors.withHashtag; {
        background = base01;
        text = base05;
        separator = base03;
        workspace = base01;
        workspace_text = base04;
        active = base0D;
        active_text = base00;
        urgent = base09;
        urgent_text = base00;
        menu = base01;
        menu_border = base03;
        menu_disabled = base03;
      };
    };
  };

  wayland.services.i3bar-wl = {
    description = "Status bar with i3 input protocol for Wayland";
    serviceConfig.ExecStart = lib.getExe pkgs.i3bar-wl;
    slice = "session";
  };
}
