{ config, inputs, ... }:
{
  imports = [ inputs.carrot.homeManagerModules.default ];

  wayland.windowManager.carrot = {
    enable = true;

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
          active_color = "#${config.lib.stylix.colors.base0D}";
          inactive_color = "#${config.lib.stylix.colors.base03}";
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

      environment = {
        CLUTTER_BACKEND = "wayland";
        GDK_BACKEND = "wayland,x11,*";
        QT_QPA_PLATFORM = "wayland;xcb";
        SDL_VIDEODRIVER = "wayland";

        _JAVA_AWT_WM_NONREPARENTING = "1";
        MOZ_ENABLE_WAYLAND = "1";
        QT_WAYLAND_DISABLE_WINDOWDECORATION = "1";
        QT_AUTO_SCREEN_SCALE_FACTOR = "1";
        WLR_NO_HARDWARE_CURSORS = "1";

        ELECTRON_OZONE_PLATFORM_HINT = "wayland";
        OZONE_PLATFORM = "wayland";
        NIXOS_OZONE_WL = "1";

        XCURSOR_THEME = config.home.pointerCursor.name;
        XCURSOR_SIZE = toString config.home.pointerCursor.size;
        GTK_THEME = config.gtk.theme.name;
        QT_QPA_PLATFORMTHEME = config.qt.platformTheme.name;
        QT_STYLE_OVERRIDE = config.qt.style.name;
      };

      binds = [
        {
          chord = "Mod+Return";
          action = "spawn";
          args = [
            "app2unit"
            "footclient"
          ];
          title = "Open a terminal";
        }
        {
          chord = "Mod+D";
          action = "spawn";
          args = [ "fuzzel" ];
          title = "App launcher";
        }
        {
          chord = "Mod+A";
          action = "spawn";
          args = [
            "swaync-client"
            "-t"
          ];
          title = "Toggle notification center";
        }
        {
          chord = "Mod+Shift+V";
          action = "spawn-sh";
          args = [ "cliphist list | fuzzel --dmenu --with-nth 2 | cliphist decode | wl-copy" ];
          title = "Paste from clipboard history";
        }
        {
          chord = "Mod+Q";
          action = "close-window";
          title = "Close window";
        }
        {
          chord = "Mod+F";
          action = "toggle-fullscreen";
          title = "Fullscreen";
        }
        {
          chord = "Mod+Space";
          action = "toggle-floating";
          title = "Float / tile";
        }
        {
          chord = "Mod+Shift+Q";
          action = "quit";
          title = "Quit carrot";
        }
        {
          chord = "Mod+Left";
          action = "focus-left";
        }
        {
          chord = "Mod+Right";
          action = "focus-right";
        }
        {
          chord = "Mod+Up";
          action = "focus-up";
        }
        {
          chord = "Mod+Down";
          action = "focus-down";
        }
        {
          chord = "Mod+H";
          action = "focus-left";
        }
        {
          chord = "Mod+J";
          action = "focus-down";
        }
        {
          chord = "Mod+K";
          action = "focus-up";
        }
        {
          chord = "Mod+L";
          action = "focus-right";
        }

        {
          chord = "Mod+Shift+Left";
          action = "swap-left";
        }
        {
          chord = "Mod+Shift+Right";
          action = "swap-right";
        }
        {
          chord = "Mod+Shift+Up";
          action = "swap-up";
        }
        {
          chord = "Mod+Shift+Down";
          action = "swap-down";
        }
        {
          chord = "Mod+Shift+H";
          action = "swap-left";
        }
        {
          chord = "Mod+Shift+J";
          action = "swap-down";
        }
        {
          chord = "Mod+Shift+K";
          action = "swap-up";
        }
        {
          chord = "Mod+Shift+L";
          action = "swap-right";
        }
        {
          chord = "Mod+Tab";
          action = "focus-next";
        }
        {
          chord = "Mod+Shift+Tab";
          action = "focus-prev";
        }
        {
          chord = "Mod+1";
          action = "focus-workspace";
          args = [ 1 ];
        }
        {
          chord = "Mod+2";
          action = "focus-workspace";
          args = [ 2 ];
        }
        {
          chord = "Mod+3";
          action = "focus-workspace";
          args = [ 3 ];
        }
        {
          chord = "Mod+4";
          action = "focus-workspace";
          args = [ 4 ];
        }
        {
          chord = "Mod+5";
          action = "focus-workspace";
          args = [ 5 ];
        }
        {
          chord = "Mod+6";
          action = "focus-workspace";
          args = [ 6 ];
        }
        {
          chord = "Mod+7";
          action = "focus-workspace";
          args = [ 7 ];
        }
        {
          chord = "Mod+8";
          action = "focus-workspace";
          args = [ 8 ];
        }
        {
          chord = "Mod+9";
          action = "focus-workspace";
          args = [ 9 ];
        }
        {
          chord = "Mod+Shift+1";
          action = "move-to-workspace";
          args = [ 1 ];
        }
        {
          chord = "Mod+Shift+2";
          action = "move-to-workspace";
          args = [ 2 ];
        }
        {
          chord = "Mod+Shift+3";
          action = "move-to-workspace";
          args = [ 3 ];
        }
        {
          chord = "Mod+Shift+4";
          action = "move-to-workspace";
          args = [ 4 ];
        }
        {
          chord = "Mod+Shift+5";
          action = "move-to-workspace";
          args = [ 5 ];
        }
        {
          chord = "Mod+Shift+6";
          action = "move-to-workspace";
          args = [ 6 ];
        }
        {
          chord = "Mod+Shift+7";
          action = "move-to-workspace";
          args = [ 7 ];
        }
        {
          chord = "Mod+Shift+8";
          action = "move-to-workspace";
          args = [ 8 ];
        }
        {
          chord = "Mod+Shift+9";
          action = "move-to-workspace";
          args = [ 9 ];
        }
        {
          chord = "Mod+Minus";
          action = "adjust-split-ratio";
          args = [ "-0.05" ];
          repeat = true;
        }
        {
          chord = "Mod+Equal";
          action = "adjust-split-ratio";
          args = [ "+0.05" ];
          repeat = true;
        }
        {
          chord = "Mod+T";
          action = "set-layout";
          args = [ "toggle" ];
          title = "Toggle layout";
        }
        {
          chord = "Mod+S";
          action = "spawn-sh";
          args = [ ''grim -g "$(slurp -o)" - | satty --filename -'' ];
          title = "Screenshot output";
        }
        {
          chord = "Mod+Shift+S";
          action = "spawn-sh";
          # FIXME: can't get focused window region
          args = [ ''grim -g "$(slurp)" - | satty --filename -'' ];
          title = "Screenshot region";
        }
        {
          chord = "XF86AudioRaiseVolume";
          action = "spawn";
          args = [
            "wpctl"
            "set-volume"
            "@DEFAULT_AUDIO_SINK@"
            "5%+"
            "--limit"
            "1.5"
          ];
          allow_when_locked = true;
        }
        {
          chord = "XF86AudioLowerVolume";
          action = "spawn";
          args = [
            "wpctl"
            "set-volume"
            "@DEFAULT_AUDIO_SINK@"
            "5%-"
            "--limit"
            "0.0"
          ];
          allow_when_locked = true;
        }
        {
          chord = "XF86AudioMute";
          action = "spawn";
          args = [
            "wpctl"
            "set-mute"
            "@DEFAULT_AUDIO_SINK@"
            "toggle"
          ];
          allow_when_locked = true;
        }
        {
          chord = "XF86AudioMicMute";
          action = "spawn";
          args = [
            "wpctl"
            "set-mute"
            "@DEFAULT_AUDIO_SOURCE@"
            "toggle"
          ];
          allow_when_locked = true;
        }

        {
          chord = "XF86AudioPlay";
          action = "spawn";
          args = [
            "playerctl"
            "play-pause"
          ];
        }
        {
          chord = "XF86AudioPause";
          action = "spawn";
          args = [
            "playerctl"
            "play-pause"
          ];
        }
        {
          chord = "XF86AudioNext";
          action = "spawn";
          args = [
            "playerctl"
            "next"
          ];
        }
        {
          chord = "XF86AudioPrev";
          action = "spawn";
          args = [
            "playerctl"
            "previous"
          ];
        }
        {
          chord = "XF86AudioStop";
          action = "spawn";
          args = [
            "playerctl"
            "stop"
          ];
        }

        # carrot-only extras: jay's shortcuts.rs doesn't bind these at all
        {
          chord = "XF86MonBrightnessUp";
          action = "spawn";
          args = [
            "brightnessctl"
            "set"
            "5%+"
          ];
          allow_when_locked = true;
        }
        {
          chord = "XF86MonBrightnessDown";
          action = "spawn";
          args = [
            "brightnessctl"
            "set"
            "5%-"
          ];
          allow_when_locked = true;
        }
      ];
    };
  };
}
