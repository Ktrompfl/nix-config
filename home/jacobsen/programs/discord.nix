{ inputs, ... }:
{
  imports = [ inputs.nixcord.homeModules.nixcord ];

  programs.nixcord = {
    enable = true;

    config = {
      disableMinSize = true;
      plugins = {
        betterGifAltText.enable = true;
        biggerStreamPreview.enable = true;
        callTimer.enable = true;
        clearUrls.enable = true;
        crashHandler.enable = true;
        disableDeepLinks.enable = true;
        fixSpotifyEmbeds.enable = true;
        fixYoutubeEmbeds.enable = true;
        forceOwnerCrown.enable = true;
        gameActivityToggle.enable = true;
        memberCount.enable = true;
        noDevtoolsWarning.enable = true;
        openInApp.enable = true;
        pinDms = {
          enable = true;
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
        spotifyShareCommands.enable = true;
        startupTimings.enable = true;
        typingIndicator.enable = true;
        userVoiceShow.enable = true;
        webContextMenus.enable = true;
        webKeybinds.enable = true;
        webScreenShareFixes.enable = true;
        youtubeAdblock.enable = true;
      };
    };

    discord = {
      krisp.enable = true;
      vencord.enable = true;
    };

    # settings = {
    #   transparencyOption = "none";
    #   tray = false;
    #   minimizeToTray = false;
    #   openLinksWithElectron = false;
    #   staticTitle = true;
    #   enableMenu = true;
    #   disableSmoothScroll = false;
    #   hardwareAcceleration = true;
    #   arRPC = true;
    #   appBadge = true;
    #   disableMinSize = true;
    #   clickTrayToShowHide = false;
    #   customTitleBar = false;
    #   enableSplashScreen = true;
    #   splashTheming = true;
    #   splashColor = config.lib.stylix.colors.withHashtag.base05;
    #   splashBackground = config.lib.stylix.colors.withHashtag.base00;
    #   spellCheckLanguages = true;
    # };
    # vencord = {
    #   settings = {
    #     autoUpdate = true;
    #     autoUpdateNotification = true;
    #     useQuickCss = true;
    #     themeLinks = [

    #     ];
    #     eagerPatches = false;
    #     enableReactDevtools = false;
    #     frameless = false;
    #     transparent = false;
    #     winCtrlQ = false;
    #     disableMinSize = false;
    #     winNativeTitleBar = false;
    #   };
    # };
  };

  # electron apps store their state inseparably in .config/
  preservation.preserveAt.state-dir.directories = [
    ".config/discord"
    ".config/vesktop"
    ".config/Vencord"
  ];
}
