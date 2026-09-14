{ config, pkgs, ... }: {
  apps.vesktop = {
    package = pkgs.vesktop;

    appId = "dev.vencord.Vesktop";

    # downloads are written straight to disk; uploads go through the portal
    access.download = "rw";

    jail.permissions =
      c: with c; [
        desktop
        network
        notifications
      ];

    files = {
      ".config/vesktop/themes/tinted.theme.css".source = pkgs.tinted-discord.themeFor config.theme.colors;

      # vesktop settings
      ".config/vesktop/settings.json" = {
        generator = (pkgs.formats.json { }).generate "vesktop-settings.json";
        value = {
          hardwareAcceleration = true;
          tray = false;
          minimizeToTray = false;
        };
      };

      # vencord settings
      ".config/vesktop/settings/settings.json" = {
        generator = (pkgs.formats.json { }).generate "vencord-settings.json";
        value = {
          autoUpdate = false;
          autoUpdateNotification = false;
          disableMinSize = true;
          enableReactDevtools = false;
          notifyAboutUpdates = false;
          useQuickCSS = false;

          enabledThemes = [ "tinted.theme.css" ];

          plugins =
            builtins.listToAttrs (
              map
                (name: {
                  inherit name;
                  value.enabled = true;
                })
                [
                  "BetterGifAltText"
                  "BiggerStreamPreview"
                  "CallTimer"
                  "ClearURLs"
                  "CrashHandler"
                  "DisableDeepLinks"
                  "FixSpotifyEmbeds"
                  "FixYoutubeEmbeds"
                  "ForceOwnerCrown"
                  "GameActivityToggle"
                  "MemberCount"
                  "NoDevtoolsWarning"
                  "OpenInApp"
                  "SpotifyShareCommands"
                  "StartupTimings"
                  "TypingIndicator"
                  "UserVoiceShow"
                  "WebContextMenus"
                  "WebKeybinds"
                  "WebScreenShareFixes"
                  "YoutubeAdblock"
                ]
            )
            // {
              PinDMs = {
                enabled = true;
                userBasedCategoryList = {
                  "139000476673769472" = [
                    {
                      channels = [
                        "336562224753672196"
                        "337256757954871311"
                        "339840044585975808"
                        "339703654107840512"
                        "337278751715098627"
                      ];
                      collapsed = false;
                      color = 2123412;
                      id = "phxnppovokp";
                      name = "C:";
                    }
                  ];
                };
              };
            };
        };
      };
    };
  };
}
