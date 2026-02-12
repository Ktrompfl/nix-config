{ config, lib, ... }:
{
  programs.vesktop = {
    enable = true;
    settings = {
      checkUpdates = false;
      discordBranch = "stable";
      transparencyOption = "none";
      tray = false;
      minimizeToTray = false;
      openLinksWithElectron = false;
      staticTitle = true;
      enableMenu = true;
      disableSmoothScroll = false;
      hardwareAcceleration = true;
      arRPC = true;
      appBadge = true;
      disableMinSize = true;
      clickTrayToShowHide = false;
      customTitleBar = false;
      enableSplashScreen = true;
      splashTheming = true;
      splashColor = config.lib.stylix.colors.withHashtag.base05;
      splashBackground = config.lib.stylix.colors.withHashtag.base00;
      spellCheckLanguages = true;
    };
    vencord = {
      settings = {
        autoUpdate = true;
        autoUpdateNotification = true;
        useQuickCss = true;
        themeLinks = [

        ];
        eagerPatches = false;
        enableReactDevtools = false;
        frameless = false;
        transparent = false;
        winCtrlQ = false;
        disableMinSize = false;
        winNativeTitleBar = false;
        plugins = {
          ChatInputButtonAPI = {
            enabled = false;
          };
          CommandsAPI = {
            enabled = true;
          };
          DynamicImageModalAPI = {
            enabled = false;
          };
          MemberListDecoratorsAPI = {
            enabled = true;
          };
          MessageAccessoriesAPI = {
            enabled = true;
          };
          MessageDecorationsAPI = {
            enabled = true;
          };
          MessageEventsAPI = {
            enabled = false;
          };
          MessagePopoverAPI = {
            enabled = false;
          };
          MessageUpdaterAPI = {
            enabled = false;
          };
          ServerListAPI = {
            enabled = false;
          };
          UserSettingsAPI = {
            enabled = true;
          };
          AccountPanelServerProfile = {
            enabled = false;
          };
          AlwaysAnimate = {
            enabled = false;
          };
          AlwaysExpandRoles = {
            enabled = false;
          };
          AlwaysTrust = {
            enabled = false;
          };
          AnonymiseFileNames = {
            enabled = false;
          };
          AppleMusicRichPresence = {
            enabled = false;
          };
          "WebRichPresence (arRPC)" = {
            enabled = false;
          };
          BetterFolders = {
            enabled = false;
          };
          BetterGifAltText = {
            enabled = true;
          };
          BetterGifPicker = {
            enabled = false;
          };
          BetterNotesBox = {
            enabled = false;
          };
          BetterRoleContext = {
            enabled = false;
          };
          BetterRoleDot = {
            enabled = false;
          };
          BetterSessions = {
            enabled = false;
            backgroundCheck = false;
            checkInterval = 20;
          };
          BetterSettings = {
            enabled = false;
          };
          BetterUploadButton = {
            enabled = false;
          };
          BiggerStreamPreview = {
            enabled = true;
          };
          BlurNSFW = {
            enabled = false;
          };
          CallTimer = {
            enabled = true;
          };
          ClearURLs = {
            enabled = true;
          };
          ClientTheme = {
            enabled = false;
          };
          ColorSighted = {
            enabled = false;
          };
          ConsoleJanitor = {
            enabled = false;
          };
          ConsoleShortcuts = {
            enabled = false;
          };
          CopyEmojiMarkdown = {
            enabled = false;
          };
          CopyFileContents = {
            enabled = false;
          };
          CopyUserURLs = {
            enabled = false;
          };
          CrashHandler = {
            enabled = true;
          };
          CtrlEnterSend = {
            enabled = false;
          };
          CustomIdle = {
            enabled = false;
          };
          CustomRPC = {
            enabled = false;
          };
          Dearrow = {
            enabled = false;
          };
          Decor = {
            enabled = false;
          };
          DisableCallIdle = {
            enabled = false;
          };
          DontRoundMyTimestamps = {
            enabled = false;
          };
          Experiments = {
            enabled = false;
          };
          ExpressionCloner = {
            enabled = false;
          };
          F8Break = {
            enabled = false;
          };
          FakeNitro = {
            enabled = false;
          };
          FakeProfileThemes = {
            enabled = false;
          };
          FavoriteEmojiFirst = {
            enabled = false;
          };
          FavoriteGifSearch = {
            enabled = false;
          };
          FixCodeblockGap = {
            enabled = false;
          };
          FixImagesQuality = {
            enabled = false;
          };
          FixSpotifyEmbeds = {
            enabled = true;
          };
          FixYoutubeEmbeds = {
            enabled = true;
          };
          ForceOwnerCrown = {
            enabled = true;
          };
          FriendInvites = {
            enabled = false;
          };
          FriendsSince = {
            enabled = true;
          };
          FullSearchContext = {
            enabled = false;
          };
          FullUserInChatbox = {
            enabled = false;
          };
          GameActivityToggle = {
            enabled = true;
          };
          GifPaste = {
            enabled = false;
          };
          GreetStickerPicker = {
            enabled = false;
          };
          HideMedia = {
            enabled = false;
          };
          iLoveSpam = {
            enabled = false;
          };
          IgnoreActivities = {
            enabled = false;
          };
          ImageLink = {
            enabled = false;
          };
          ImageZoom = {
            enabled = false;
          };
          ImplicitRelationships = {
            enabled = false;
          };
          InvisibleChat = {
            enabled = false;
          };
          IrcColors = {
            enabled = false;
          };
          KeepCurrentChannel = {
            enabled = false;
          };
          LastFMRichPresence = {
            enabled = false;
          };
          LoadingQuotes = {
            enabled = false;
          };
          MemberCount = {
            enabled = true;
          };
          MentionAvatars = {
            enabled = false;
          };
          MessageClickActions = {
            enabled = false;
          };
          MessageLatency = {
            enabled = false;
          };
          MessageLinkEmbeds = {
            enabled = false;
          };
          MessageLogger = {
            enabled = false;
          };
          MessageTags = {
            enabled = false;
          };
          MutualGroupDMs = {
            enabled = false;
          };
          NewGuildSettings = {
            enabled = false;
          };
          NoBlockedMessages = {
            enabled = false;
          };
          NoDevtoolsWarning = {
            enabled = true;
          };
          NoF1 = {
            enabled = false;
          };
          NoMaskedUrlPaste = {
            enabled = false;
          };
          NoMosaic = {
            enabled = false;
          };
          NoOnboardingDelay = {
            enabled = false;
          };
          NoPendingCount = {
            enabled = false;
          };
          NoProfileThemes = {
            enabled = false;
          };
          NoReplyMention = {
            enabled = false;
          };
          NoServerEmojis = {
            enabled = false;
          };
          NoTypingAnimation = {
            enabled = false;
          };
          NoUnblockToJump = {
            enabled = false;
          };
          NormalizeMessageLinks = {
            enabled = false;
          };
          NotificationVolume = {
            enabled = false;
          };
          OnePingPerDM = {
            enabled = false;
          };
          oneko = {
            enabled = false;
          };
          OpenInApp = {
            enabled = true;
          };
          OverrideForumDefaults = {
            enabled = false;
          };
          PauseInvitesForever = {
            enabled = false;
          };
          PermissionFreeWill = {
            enabled = false;
          };
          PermissionsViewer = {
            enabled = false;
          };
          petpet = {
            enabled = false;
          };
          PictureInPicture = {
            enabled = false;
          };
          PinDMs = {
            enabled = true;
            canCollapseDmSection = false;
            userBasedCategoryList = {
              "139000476673769472" = [
                {
                  id = "phxnppovokp";
                  name = "C:";
                  color = 2123412;
                  collapsed = false;
                  channels = [
                    "336562224753672196"
                    "337256757954871311"
                    "339840044585975808"
                    "339703654107840512"
                    "337278751715098627"
                  ];
                }
              ];
            };
            pinOrder = 0;
          };
          PlainFolderIcon = {
            enabled = false;
          };
          PlatformIndicators = {
            enabled = false;
          };
          PreviewMessage = {
            enabled = false;
          };
          QuickMention = {
            enabled = false;
          };
          QuickReply = {
            enabled = false;
          };
          ReactErrorDecoder = {
            enabled = false;
          };
          ReadAllNotificationsButton = {
            enabled = false;
          };
          RelationshipNotifier = {
            enabled = false;
          };
          ReplaceGoogleSearch = {
            enabled = false;
          };
          ReplyTimestamp = {
            enabled = false;
          };
          RevealAllSpoilers = {
            enabled = false;
          };
          ReverseImageSearch = {
            enabled = false;
          };
          ReviewDB = {
            enabled = false;
          };
          RoleColorEverywhere = {
            enabled = false;
          };
          SecretRingToneEnabler = {
            enabled = false;
          };
          Summaries = {
            enabled = false;
          };
          SendTimestamps = {
            enabled = false;
          };
          ServerInfo = {
            enabled = false;
          };
          ServerListIndicators = {
            enabled = false;
          };
          ShikiCodeblocks = {
            enabled = false;
          };
          ShowAllMessageButtons = {
            enabled = false;
          };
          ShowConnections = {
            enabled = false;
          };
          ShowHiddenChannels = {
            enabled = false;
          };
          ShowHiddenThings = {
            enabled = false;
          };
          ShowMeYourName = {
            enabled = false;
          };
          ShowTimeoutDuration = {
            enabled = false;
          };
          SilentMessageToggle = {
            enabled = false;
          };
          SilentTyping = {
            enabled = false;
          };
          SortFriendRequests = {
            enabled = false;
          };
          SpotifyControls = {
            enabled = false;
          };
          SpotifyCrack = {
            enabled = false;
          };
          SpotifyShareCommands = {
            enabled = true;
          };
          StartupTimings = {
            enabled = true;
          };
          StickerPaste = {
            enabled = false;
          };
          StreamerModeOnStream = {
            enabled = false;
          };
          SuperReactionTweaks = {
            enabled = false;
          };
          TextReplace = {
            enabled = false;
          };
          ThemeAttributes = {
            enabled = false;
          };
          Translate = {
            enabled = false;
          };
          TypingIndicator = {
            enabled = true;
          };
          TypingTweaks = {
            enabled = false;
          };
          Unindent = {
            enabled = false;
          };
          UnlockedAvatarZoom = {
            enabled = false;
          };
          UnsuppressEmbeds = {
            enabled = false;
          };
          UserMessagesPronouns = {
            enabled = false;
          };
          UserVoiceShow = {
            enabled = true;
          };
          USRBG = {
            enabled = false;
          };
          ValidReply = {
            enabled = false;
          };
          ValidUser = {
            enabled = false;
          };
          VoiceChatDoubleClick = {
            enabled = false;
          };
          VcNarrator = {
            enabled = false;
          };
          VencordToolbox = {
            enabled = false;
          };
          ViewIcons = {
            enabled = false;
          };
          ViewRaw = {
            enabled = false;
          };
          VoiceDownload = {
            enabled = false;
          };
          VoiceMessages = {
            enabled = false;
          };
          VolumeBooster = {
            enabled = false;
          };
          WebKeybinds = {
            enabled = true;
          };
          WebScreenShareFixes = {
            enabled = true;
          };
          WhoReacted = {
            enabled = false;
          };
          XSOverlay = {
            enabled = false;
          };
          YoutubeAdblock = {
            enabled = true;
          };
          BadgeAPI = {
            enabled = true;
          };
          NoTrack = {
            enabled = true;
            disableAnalytics = true;
          };
          Settings = {
            enabled = true;
            settingsLocation = "aboveNitro";
          };
          DisableDeepLinks = {
            enabled = true;
          };
          SupportHelper = {
            enabled = true;
          };
          WebContextMenus = {
            enabled = true;
          };
        };
        notifications = {
          timeout = 5000;
          position = "bottom-right";
          useNative = "not-focused";
          logLimit = 50;
        };
        cloud = {
          authenticated = false;
          url = "https://api.vencord.dev/";
          settingsSync = false;
          settingsSyncVersion = 1750589615380;
        };
      };
    };
  };

  programs.vesktop.vencord.themes.stylix = lib.mkAfter /* css */ ''
    /*
     * RemoveButtons.theme.css by cheesits456 (https://cheesits456.dev)
     * For CSS compatible with browser extensions, see
     * https://gist.github.com/cheesits456/0d5bede837f022e443e9a5fc4aa386cb#file-discordbrowserstyles-css-L1
    */

    /* Hide Nitro gift button */
    button[aria-label="Send a gift"] {
        display: none;
    }
    /* Hide GIF picker button */
    /*
    button[aria-label="Open GIF picker"] {
        display: none;
    }
    */
    /* Hide sticker picker button */
    button[aria-label="Open sticker picker"] {
        display: none;
    }
    /* Hide annoying sticker popup window that appears when you type */
    .da-channelTextArea > .container-JHR0NT {
        display: none;
    }
    /* Hide emoji picker button */
    button[aria-label="Select emoji"] {
        display: none;
    }
    /* Hide stickers tab in emoji selector */
    [aria-controls="sticker-picker-tab-panel"]
    {
        display: none;
    }
    /* Hide Nitro button in server list */
    .fixedBottomList-1yrBla {
        display: none;
    }
    /* Hide more annoying Nitro buttons */
    ul > li[role="listitem"] a[href="/library"],
    ul > li[role="listitem"] a[href="/store"] {
            display: none;
    }
  '';

  # electron apps store their state inseparably in .config/
  preservation.preserveAt.state-dir.directories = [
    ".config/discord"
    ".config/vesktop"
  ];
}
