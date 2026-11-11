{
  config,
  lib,
  osConfig,
  pkgs,
  ...
}:
{
  packages = [ pkgs.i3status-rust ];

  xdg.config.files."i3status-rust/config-jay.toml" = {
    generator = (pkgs.formats.toml { }).generate "i3status-rust-jay.toml";
    value =
      with config.theme.colors.withHashtag;
      let
        # icons from the proportional nerd font, dimmed so the values stand
        # out; they keep the block's color, so they follow its state
        glyph = text: "<span font_family='${config.theme.fonts.symbols.name}' fgalpha='60%'>${text}</span>";
        # for things that need attention
        alert =
          text: "<span font_family='${config.theme.fonts.symbols.name}' foreground='${error}'>${text}</span>";
        icon = glyph "$icon";
        # numbers padded to two digits with figure spaces (U+2007), which are
        # as wide as a digit, so values keep their width as they change
        figureSpace = builtins.fromJSON ''" "'';
        # tnum: tabular figures, all digits equally wide
        number =
          args: name:
          "<span font_features='tnum'>$" + "${name}.eng(${args}, pad_with:'${figureSpace}')</span>";
        num = number "w:2";
        # rates from 0 to 999 per unit, at least in kilobytes; only the prefix
        # (k, M, G) is shown, the arrows already say it is a rate
        speed = number "w:3, prefix:K, hide_unit:true";
        # the blocks carry their own spacing: a gap between the items of a
        # group, a divider between groups
        gap = "  ";
        divider = "  <span foreground='${base03}'>│</span>  ";

        hasRptuVpn = osConfig.systemd.network.netdevs ? "30-wg-rptu";

        # one entry per device; the path is any mount on it
        disks = [
          {
            # the tmpfs root, as a plain dimmed slash in the text font
            label = "<span fgalpha='60%'>/</span>";
            path = "/";
          }
          {
            # the main pool, shared by /persist, /nix, /cache, /var/log
            label = glyph "󰋊";
            path = "/persist";
          }
        ]
        ++ lib.optional (osConfig.fileSystems ? "/extra") {
          label = glyph "󰆼";
          path = "/extra";
        };

        # the number of failed system and user units, nothing if there are none
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

          # the gpu: load, temperature, vram
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
        # a group of its own, each disk with its label
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
        # the rptu wireguard tunnel, shown while it is up
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
            # the date dimmed next to the time
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
            # only while something captures the microphone, screen or camera
            block = "privacy";
            driver = [ { name = "pipewire"; } ];
            format = lib.concatMapStrings (kind: "{${gap}${alert "$icon_${kind}"}|}") [
              "audio"
              "audio_sink"
              "video"
              "webcam"
              "unknown"
            ];
          }

          {
            # only while units have failed
            block = "custom";
            command = "${failedUnits}";
            interval = 60;
            hide_when_empty = true;
            format = "${gap}${alert "󰀦"} <span foreground='${base08}'>$text</span>";
          }

          {
            block = "custom_dbus";
            path = "/idle_inhibitor";
            format = {
              # trailing gap: the tray follows
              full = "${gap}${glyph "{$text.pango-str()|󰾪}"}${gap}";
              short = "";
            };
          }
        ];
      };
  };
}
