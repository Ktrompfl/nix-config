{
  config,
  lib,
  pkgs,
  ...
}:
{
  wayland.windowManager.sway = {
    enable = true;
    wrapperFeatures.gtk = true;
    xwayland = true;
    checkConfig = false; # broken
    config =
      let
        cfg = config.wayland.windowManager.sway.config;
      in
      {
        # default applications
        menu = "fuzzel";
        terminal = "footclient";

        input = {
          # laptop keyboard layout, german for now
          "1:1:AT_Translated_Set_2_keyboard" = {
            xkb_layout = "de";
            xkb_variant = "qwerty"; # swap y and z
            xkb_options = "caps:escape";
            xkb_model = "pc105"; # iso
          };

          "1389:49271:I-CHIP_YUNZII_AL68_2.4G" = {
            xkb_layout = "us";
            xkb_variant = "altgr-intl";
            xkb_options = "caps:escape";
            xkb_model = "pc104"; # ansi
          };

          "1389:49271:I-CHIP_YUNZII_AL68_2.4G_Keyboard" = {
            xkb_layout = "us";
            xkb_variant = "altgr-intl";
            xkb_options = "caps:escape";
            xkb_model = "pc104"; # ansi
          };

          "type:pointer" = {
            accel_profile = "flat";
            natural_scroll = "disabled";
            pointer_accel = "0"; # in [-1,1]
          };

          "type:touchpad" = {
            accel_profile = "flat";
            dwt = "disabled"; # disable while typing -- this breaks various games :/
            middle_emulation = "enabled";
            natural_scroll = "disabled";
          };
        };

        # keybindings
        modifier = "Mod4"; # SUPER
        left = "h";
        right = "l";
        up = "k";
        down = "j";
        keybindings =
          let
            launch-prefix = "${lib.getExe pkgs.app2unit} --";
            grimblast = "${lib.getExe pkgs.grimblast}";
            wl-present = "${pkgs.wl-mirror}/bin/wl-present";

            screenshot-dir = "~/Pictures/screenshots/$(date +'%Y-%m-%d-%H%M%S.png')";
            focused-monitor = ''"$(swaymsg -t get_outputs | ${lib.getExe pkgs.jq} -r '.[] | select(.focused == true).name')"'';
          in
          {
            # window controls
            "${cfg.modifier}+q" = "kill";

            "${cfg.modifier}+h" = "focus left";
            "${cfg.modifier}+j" = "focus down";
            "${cfg.modifier}+k" = "focus up";
            "${cfg.modifier}+l" = "focus right";

            "${cfg.modifier}+Shift+h" = "move left";
            "${cfg.modifier}+Shift+j" = "move down";
            "${cfg.modifier}+Shift+k" = "move up";
            "${cfg.modifier}+Shift+l" = "move right";

            "${cfg.modifier}+space" = "floating toggle";
            "${cfg.modifier}+f" = "fullscreen toggle";
            "${cfg.modifier}+v" = "layout toggle splitv splith";

            # workspace controls
            "${cfg.modifier}+ctrl+h" = "workspace prev";
            "${cfg.modifier}+ctrl+l" = "workspace next";

            "${cfg.modifier}+1" = "workspace number 1";
            "${cfg.modifier}+2" = "workspace number 2";
            "${cfg.modifier}+3" = "workspace number 3";
            "${cfg.modifier}+4" = "workspace number 4";
            "${cfg.modifier}+5" = "workspace number 5";
            "${cfg.modifier}+6" = "workspace number 6";
            "${cfg.modifier}+7" = "workspace number 7";
            "${cfg.modifier}+8" = "workspace number 8";
            "${cfg.modifier}+9" = "workspace number 9";
            "${cfg.modifier}+0" = "workspace number 10";

            "${cfg.modifier}+Shift+1" = "move container to workspace number 1";
            "${cfg.modifier}+Shift+2" = "move container to workspace number 2";
            "${cfg.modifier}+Shift+3" = "move container to workspace number 3";
            "${cfg.modifier}+Shift+4" = "move container to workspace number 4";
            "${cfg.modifier}+Shift+5" = "move container to workspace number 5";
            "${cfg.modifier}+Shift+6" = "move container to workspace number 6";
            "${cfg.modifier}+Shift+7" = "move container to workspace number 7";
            "${cfg.modifier}+Shift+8" = "move container to workspace number 8";
            "${cfg.modifier}+Shift+9" = "move container to workspace number 9";
            "${cfg.modifier}+Shift+0" = "move container to workspace number 10";

            # modes
            "${cfg.modifier}+p" = "mode power";
            "${cfg.modifier}+r" = "mode resize";

            # applications
            "${cfg.modifier}+a" = "exec swaync-client -t";
            "${cfg.modifier}+Return" = "exec ${launch-prefix} ${cfg.terminal}";
            "${cfg.modifier}+d" = "exec ${cfg.menu}";

            # screenshot
            "${cfg.modifier}+s" = "exec ${grimblast} --freeze --notify copysave screen ${screenshot-dir}";
            "${cfg.modifier}+shift+s" = "exec ${grimblast} --freeze --notify copysave area ${screenshot-dir}";

            # mirror displays with wl-mirror / wl-present
            "${cfg.modifier}+m" = "exec ${wl-present} mirror ${focused-monitor}";
            "${cfg.modifier}+z" = "exec ${wl-present} toggle-freeze";
          };

        modes = {
          power = {
            "l" = "exec ${lib.getExe pkgs.swaylock}, mode default";
            "s" = "exec systemctl poweroff";
            "r" = "exec systemctl reboot";
            "h" = "exec systemctl suspend, mode default";

            # return to default mode
            "Escape" = "mode default";
            "${cfg.modifier}+p" = "mode default";
          };
          resize = {
            # Binds arrow keys to resizing commands
            "${cfg.left}" = "resize shrink width 10 px";
            "${cfg.down}" = "resize grow height 10 px";
            "${cfg.up}" = "resize shrink height 10 px";
            "${cfg.right}" = "resize grow width 10 px";

            # return to default mode
            "Escape" = "mode default";
            "${cfg.modifier}+r" = "mode default";
          };
        };

        bars = [ ]; # now bars

        window = {
          border = 1;
          titlebar = false;
        };
      };
  };
}
