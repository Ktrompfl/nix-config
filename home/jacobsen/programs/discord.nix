{ config, pkgs, ... }:
let
  enabled = [
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
  ];

  vencordSettings = {
    autoUpdate = false;
    autoUpdateNotification = false;
    disableMinSize = true;
    enableReactDevtools = false;
    notifyAboutUpdates = false;
    useQuickCSS = false;

    enabledThemes = [ "tinted.theme.css" ];

    plugins =
      builtins.listToAttrs (
        map (name: {
          inherit name;
          value.enabled = true;
        }) enabled
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
in
{
  packages = [ pkgs.vesktop ];

  xdg.config.files = {
    "vesktop/themes/tinted.theme.css".source = pkgs.tinted-discord.themeFor config.theme.colors;

    # vesktop settings
    "vesktop/settings.json" = {
      generator = (pkgs.formats.json { }).generate "vesktop-settings.json";
      value = {
        hardwareAcceleration = true;
        tray = false;
        minimizeToTray = false;
      };
    };

    # vencord settings
    "vesktop/settings/settings.json" = {
      generator = (pkgs.formats.json { }).generate "vencord-settings.json";
      value = vencordSettings;
    };
  };

  preservation.preserveAt.state-dir.directories = [
    ".config/vesktop"
  ];
}
