{ config, ... }:
{
  # Generates the status text fed into jay's built-in bar (see
  # ../wayland/jay-config/src/status.rs for how it's wired in). jay's bar
  # otherwise natively handles workspaces, the tray (via wl-tray-bridge,
  # see ../wayland/wl-tray-bridge.nix) and the clock.
  programs.i3status-rust = {
    enable = true;
    bars.jay = {
      icons = "awesome6";

      # NOTE: `programs.i3status-rust` merges `settings` on top of its own
      # generated config with `//`, a shallow merge, so `settings.theme` has
      # to repeat `theme` here or it would clobber the `bars.jay.theme` option.
      settings.theme = {
        theme = "plain";
        overrides = with config.lib.stylix.colors.withHashtag; {
          idle_bg = base02;
          idle_fg = base04;
          info_bg = base0D;
          info_fg = base00;
          good_bg = base0B;
          good_fg = base00;
          warning_bg = base09;
          warning_fg = base00;
          critical_bg = base08;
          critical_fg = base00;
          separator = " ";
        };
      };

      blocks = [
        # mode indicator, mirrors waybar's "custom/mode" (updated by the
        # same jay-mode script that also signals waybar, see ../wayland/jay.nix)
        {
          block = "custom";
          command = ''(line=$(tail -n1 "$XDG_RUNTIME_DIR/waybar/custom/mode" 2>/dev/null); [ -z "$line" ] && echo "" || echo "$line" | tr '[:lower:]' '[:upper:]')'';
          signal = 1;
          interval = "once";
          hide_when_empty = true;
        }

        # focused window title, mirrors waybar's "sway/window"
        {
          block = "focused_window";
          driver = "wlr_toplevel_management";
          format = " $title.str(max_w:64) ";
        }

        {
          block = "cpu";
          interval = 5;
          format = " $icon $utilization ";
        }
        {
          block = "memory";
          interval = 10;
          format = " $icon $mem_used_percents ";
        }
        {
          block = "disk_space";
          path = "/persist/";
          info_type = "used";
          interval = 30;
          format = " $icon $percentage ";
        }

        {
          block = "net";
          interval = 5;
        }
        {
          block = "backlight";
        }
        {
          block = "battery";
          format = " $icon $percentage ";
        }
        {
          block = "sound";
        }

        # notification indicator, mirrors waybar's "custom/swaync"
        # NOTE: unlike waybar's version this can't be clicked to open the
        # control center, since jay's status text has no click support
        {
          block = "custom";
          command = "swaync-client -swb | jq -r .text";
          interval = 5;
          hide_when_empty = true;
        }
      ];
    };
  };
}
