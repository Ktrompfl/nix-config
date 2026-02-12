{
  config,
  lib,
  pkgs,
  ...
}:
{
  # setup firefox as default browser via session variables
  home.sessionVariables = {
    BROWSER = "${lib.getExe pkgs.firefox}";
    DEFAULT_BROWSER = "${lib.getExe pkgs.firefox}"; # for electron apps
  };

  stylix.targets.firefox = {
    profileNames = [ "jacobsen" ];
    colorTheme.enable = true;
    firefoxGnomeTheme.enable = false;
  };

  programs = {
    firefox = {
      enable = true;
      nativeMessagingHosts = [ pkgs.tridactyl-native ];
      policies = {
        # debloat
        AppAutoUpdate = false;
        CaptivePortal = false;
        DisableAppUpdate = false;
        DisableDefaultBrowserAgent = true;
        DisableFirefoxAccounts = true;
        DisableFirefoxStudies = true;
        DisablePocket = true;
        DisableProfileImport = true;
        DisableTelemetry = true;
        DontCheckDefaultBrowser = true;
        NetworkPrediction = false;
        NoDefaultBookmarks = true;
        OfferToSaveLogins = false;
        OfferToSaveLoginsDefault = false;
        SearchSuggestEnabled = false;
        PasswordManagerEnabled = false;
        FirefoxHome = {
          Search = true;
          Pocket = false;
          Snippets = false;
          TopSites = false;
          Highlights = false;
          SponsoredTopSites = false;
          SponsoredPocket = false;
          Locked = true;
        };
        ShowHomeButton = false;
        UserMessaging = {
          ExtensionRecommendations = false;
          UrlbarInterventions = false;
          SkipOnboarding = true;
          MoreFromMozilla = false;
          FirefoxLabs = true;
        };
        FirefoxSuggest = {
          WebSuggestions = false;
          SponsoredSuggestions = false;
          ImproveSuggest = false;
          Locked = true;
        };

        # security
        AutofillAddressEnabled = false;
        AutofillCreditCardEnabled = false;
        HttpsOnlyMode = "force_enabled";
        SSLVersionMin = "tls1.2";
        PostQuantumKeyAgreementEnabled = true;
        HttpAllowlist = [
          "http://localhost"
          "http://127.0.0.1"
        ];
      };
      # package = pkgs.wrapFirefox pkgs.firefox-unwrapped { extraPolicies = { }; };
      profiles = {
        jacobsen = {
          isDefault = true;

          settings = {
            # debloat
            "browser.discovery.enabled" = false;
            "app.shield.optoutstudies.enabled" = false;
            "browser.topsites.contile.enabled" = false;
            "browser.urlbar.suggest.quicksuggest.sponsored" = false;
            "browser.urlbar.trending.featureGate" = false;
            "browser.newtabpage.activity-stream.feeds.section.topstories" = false;
            "browser.newtabpage.activity-stream.feeds.snippets" = false;
            "browser.newtabpage.activity-stream.section.highlights.includePocket" = false;
            "browser.newtabpage.activity-stream.section.highlights.includeBookmarks" = false;
            "browser.newtabpage.activity-stream.section.highlights.includeDownloads" = false;
            "browser.newtabpage.activity-stream.section.highlights.includeVisited" = false;
            "browser.newtabpage.activity-stream.showSponsored" = false;
            "browser.newtabpage.activity-stream.system.showSponsored" = false;
            "browser.newtabpage.activity-stream.showSponsoredTopSites" = false;
            # Privacy: Disable automatic opening in new windows (manually still works)
            # https://gitlab.torproject.org/tpo/applications/tor-browser/-/issues/9881
            "browser.link.open_newwindow" = 3;
            # Privacy: Set all window open modes to abide above method
            "browser.link.open_newwindow.restriction" = 0;

            # privacy
            "privacy.resistFingerprinting" = "true";
            # disable sending downloaded files to the internet
            "browser.safebrowsing.downloads.remote.enabled" = false;
            "network.dns.disablePrefetch" = false;
            # redundancy: disable network prefetching
            "network.predictor.enabled" = false;
            # disable preloading websites when hovering over links
            "network.http.speculative-parallel-limit" = 0;
            # disable connecting to bookmarks when hovering over them
            "browser.places.speculativeConnect.enabled" = "false";
            "privacy.globalprivacycontrol.enabled" = true;
            "privacy.clearOnShutdown_v2.cookiesAndStorage" = true;
            "privacy.fingerprintingProtection" = true;

            "browser.contentblocking.category" = "strict";
            "extensions.pocket.enabled" = false;
            "browser.search.suggest.enabled" = false;
            "browser.search.suggest.enabled.private" = false;
            "browser.urlbar.suggest.searches" = false;
            # store media in cache only on private browsing
            "browser.privatebrowsing.forceMediaMemoryCache" = true;
            "network.http.referer.XOriginTrimmingPolicy" = 2;
            # Privacy: Disable CSP reporting
            # https://bugzilla.mozilla.org/show_bug.cgi?id=1964249
            "security.csp.reporting.enabled" = false;

            # security
            #"browser.formfill.enable" = false;
            "pdfjs.enableScripting" = false;
            #"signon.autofillForms" = false
            # UNCLEAR
            "signon.formlessCapture.enabled" = false;
            # prevent scripts from moving or resizing windows
            "dom.disable_window_move_resize" = true;
            # Security: Disable remote debugging feature
            # https://gitlab.torproject.org/tpo/applications/tor-browser/-/issues/16222
            "devtools.debugger.remote-enabled" = false;
            # Security: Restrict directories from which extensions can be loaded (Unclear)
            # https://archive.is/DYjAM
            # "extensions.enabledScopes" = 5;

            # ssl
            # Security: Require safe SSL negotiation to avoid potentially MITMed sites
            "security.ssl.require_safe_negotiation" = true;
            # Security: Disable TLS1.3 0-RTT as key encryption may not be forward secret
            # https://github.com/tlswg/tls13-spec/issues/1001
            "security.tls.enable_0rtt_data" = 2;
            # Security: Enable strict public key pinning, prevents some MITM attacks
            "security.cert_pinning.enforcement_level" = 2;
            # Security: Enable CRLite to ensure that revoked certificates are detected
            "security.pki.crlite_mode" = 2;
            # Security: Treat unsafe negotiation as broken
            # https://wiki.mozilla.org/Security:Renegotiation
            # https://bugzilla.mozilla.org/1353705
            "security.ssl.treat_unsafe_negotiation_as_broken" = true;
            #  Security: Display more information on Insecure Connection warning pages
            # Test: https://badssl.com
            "browser.xul.error_pages.expert_bad_cert" = true;

            # features
            "layout.spellcheckDefault" = 1;
            # Use the systems native filechooser portal
            "widget.use-xdg-desktop-portal.file-picker" = 1;
            # allow adblockers to act everywhere. WARNING this is a security hole.
            "extensions.webextensions.restrictedDomains" = "";
            "media.webrtc.camera.allow-pipewire" = true;
            "browser.download.always_ask_before_handling_new_types" = true;

            # ui
            "browser.startup.page" = 3; # restore previous session
            "browser.newtabpage.enabled" = false;
            "browser.tabs.inTitlebar" = 0;
            "browser.toolbars.bookmarks.visibility" = "never";
            "browser.uiCustomization.state" = {
              placements = {
                widget-overflow-fixed-list = [

                ];
                unified-extensions-area = [
                  "languagetool-webextension_languagetool_org-browser-action"
                  "gdpr_cavi_au_dk-browser-action"
                  "firefoxcolor_mozilla_com-browser-action"
                  "_076d8ebb-5df6-48e0-a619-99315c395644_-browser-action"
                ];
                nav-bar = [
                  "back-button"
                  "forward-button"
                  "stop-reload-button"
                  "vertical-spacer"
                  "urlbar-container"
                  "downloads-button"
                  "unified-extensions-button"
                  "reset-pbm-toolbar-button"
                  "ublock0_raymondhill_net-browser-action"
                  "_446900e4-71c2-419f-a6a7-df9c091e268b_-browser-action"
                ];
                toolbar-menubar = [ "menubar-items" ];
                TabsToolbar = [
                  "tabbrowser-tabs"
                  "new-tab-button"
                  "alltabs-button"
                ];
                vertical-tabs = [

                ];
                PersonalToolbar = [

                ];
              };
              seen = [
                "developer-button"
                "screenshot-button"
                "languagetool-webextension_languagetool_org-browser-action"
                "_446900e4-71c2-419f-a6a7-df9c091e268b_-browser-action"
                "gdpr_cavi_au_dk-browser-action"
                "firefoxcolor_mozilla_com-browser-action"
                "_076d8ebb-5df6-48e0-a619-99315c395644_-browser-action"
                "ublock0_raymondhill_net-browser-action"
              ];
              dirtyAreaCache = [
                "nav-bar"
                "toolbar-menubar"
                "TabsToolbar"
                "vertical-tabs"
                "PersonalToolbar"
                "unified-extensions-area"
              ];
              currentVersion = 23;
              newElementCount = 7;
            };
            "extensions.autoDisableScopes" = 0; # automatically enable extensions
            "extensions.update.autoUpdateDefault" = false;
            "extensions.update.enabled" = false;
          };

          extensions = {
            force = true;
            exactPermissions = true;
            exhaustivePermissions = true;

            packages = with pkgs.nur.repos.rycee.firefox-addons; [
              bitwarden # password manager
              darkreader # dark mode for every website
              languagetool # spell/grammar checker
              consent-o-matic # automatically handle gdpr consent forms
              ublock-origin # ad blocker
              tridactyl
            ];

            # see https://developer.mozilla.org/en-US/docs/Mozilla/Add-ons/WebExtensions/manifest.json/permissions for permissions
            settings = {
              # firefox color (installed by stylix)
              "FirefoxColor@mozilla.com".permissions = [
                "theme"
                "storage"
                "tabs"
                "https://color.firefox.com/*"
              ];
              # bitwarden
              "{446900e4-71c2-419f-a6a7-df9c091e268b}".permissions = [
                "<all_urls>"
                "*://*/*"
                "alarms"
                "clipboardRead"
                "clipboardWrite"
                "contextMenus"
                "idle"
                "storage"
                "tabs"
                "unlimitedStorage"
                "webNavigation"
                "webRequest"
                "webRequestBlocking"
                "notifications"
                "file:///*"
              ];
              # languagetool
              "languagetool-webextension@languagetool.org".permissions = [
                "activeTab"
                "storage"
                "contextMenus"
                "scripting"
                "alarms"
                "http://*/*"
                "https://*/*"
                "file:///*"
                "*://docs.google.com/document/*"
                "*://docs.google.com/presentation/*"
                "*://languagetool.org/*"
                "https://languagetool.org/*/webextension/premium-announcement*"
                "https://languagetool.org/webextension/premium-announcement*"
                "http://localhost:8000/*/webextension/premium-announcement*"
                "http://localhost:8000/webextension/premium-announcement*"
              ];
              # consent-o-matic
              "gdpr@cavi.au.dk".permissions = [
                "activeTab"
                "tabs"
                "storage"
                "<all_urls>"
              ];
              # dark reader
              "addon@darkreader.org".permissions = [
                "alarms"
                "contextMenus"
                "storage"
                "tabs"
                "theme"
                "<all_urls>"
              ];
              "addon@darkreader.org".settings.theme = with config.lib.stylix.colors.withHashtag; {
                fontFamily = config.stylix.fonts.sansSerif.name;
                lightSchemeBackgroundColor = base00;
                darkSchemeBackgroundColor = base00;
                lightSchemeTextColor = base05;
                darkSchemeTextColor = base05;
                selectionColor = base0D;
              };
              # ublock-origin
              "uBlock0@raymondhill.net".permissions = [
                "alarms"
                "dns"
                "menus"
                "privacy"
                "storage"
                "tabs"
                "unlimitedStorage"
                "webNavigation"
                "webRequest"
                "webRequestBlocking"
                "<all_urls>"
                "http://*/*"
                "https://*/*"
                "file://*/*"
                "https://easylist.to/*"
                "https://*.fanboy.co.nz/*"
                "https://filterlists.com/*"
                "https://forums.lanik.us/*"
                "https://github.com/*"
                "https://*.github.io/*"
                "https://github.com/uBlockOrigin/*"
                "https://ublockorigin.github.io/*"
                "https://*.reddit.com/r/uBlockOrigin/*"
              ];
              # tridactyl
              "tridactyl.vim@cmcaine.co.uk".permissions = [
                "activeTab"
                "bookmarks"
                "browsingData"
                "contextMenus"
                "contextualIdentities"
                "cookies"
                "clipboardWrite"
                "clipboardRead"
                "downloads"
                "find"
                "history"
                "search"
                "sessions"
                "storage"
                "tabHide"
                "tabs"
                "topSites"
                "management"
                "nativeMessaging"
                "webNavigation"
                "webRequest"
                "webRequestBlocking"
                "proxy"
                "<all_urls>"
              ];
            };
          };
          userChrome = /* css */ ''
            /* hide window controls */
            @namespace url("http://www.mozilla.org/keymaster/gatekeeper/there.is.only.xul"); /* only needed once */

            .titlebar-min {display:none!important;}
            .titlebar-max {display:none!important;}
            .titlebar-restore {display:none!important;}
            .titlebar-close {display:none!important;}
          '';

          search = {
            force = true;
            default = "google";
            order = [ "google" ];
            engines = {
              google.metaData.alias = "@g";
              wikipedia.metaData.alias = "@wiki";
              bing.metaData.hidden = true;
              duckduckgo.metaData.hidden = true;

              "Nix Packages" = {
                urls = [
                  {
                    template = "https://search.nixos.org/packages?type=packages&channel=unstable&query={searchTerms}";
                  }
                ];
                icon = "${pkgs.nixos-icons}/share/icons/hicolor/scalable/apps/nix-snowflake.svg";
                definedAliases = [ "@np" ];
              };

              "NixOS Wiki" = {
                urls = [
                  { template = "https://wiki.nixos.org/index.php?search={searchTerms}&title=Special%3ASearch"; }
                ];
                icon = "${pkgs.nixos-icons}/share/icons/hicolor/scalable/apps/nix-snowflake.svg";
                updateInterval = 24 * 60 * 60 * 1000; # every day
                definedAliases = [ "@nw" ];
              };

              "Nix Options" = {
                definedAliases = [ "@no" ];
                icon = "${pkgs.nixos-icons}/share/icons/hicolor/scalable/apps/nix-snowflake.svg";
                urls = [ { template = "https://search.nixos.org/options?query={searchTerms}"; } ];
              };

              "Home Manager Options" = {
                definedAliases = [ "@hm" ];
                icon = "${pkgs.nixos-icons}/share/icons/hicolor/scalable/apps/nix-snowflake.svg";
                urls = [
                  { template = "https://home-manager-options.extranix.com/?query={searchTerms}&release=master"; }
                ];
              };

              "Youtube" = {
                definedAliases = [ "@yt" ];
                urls = [ { template = "https://youtube.com/search?q={searchTerms}"; } ];
                iconMapObj."16" = "https://www.youtube.com/s/desktop/606e092f/img/logos/favicon.ico";
              };

              "PyPI" = {
                definedAliases = [ "@py" ];
                urls = [
                  {
                    template = "https://pypi.org/search/";
                    params = [
                      {
                        name = "q";
                        value = "{searchTerms}";
                      }
                    ];
                  }
                ];
                iconMapObj."16" = "https://pypi.org/static/images/favicon.35549fe8.ico";
              };

              "GitHub Code Search" = {
                definedAliases = [ "@gh" ];
                urls = [
                  {
                    template = "https://github.com/search";
                    params = [
                      {
                        name = "type";
                        value = "code";
                      }
                      {
                        name = "q";
                        value = "{searchTerms}";
                      }
                    ];
                  }
                ];
              };
            };
          };
        };
      };
    };
  };

  xdg.mimeApps.defaultApplications = {
    "x-scheme-handler/http" = "firefox.desktop";
    "x-scheme-handler/https" = "firefox.desktop";
    "x-scheme-handler/about" = "firefox.desktop";
    "x-scheme-handler/unknown" = "firefox.desktop";
    "x-scheme-handler/chrome" = "firefox.desktop";
    "application/x-extension-htm" = "firefox.desktop";
    "application/x-extension-html" = "firefox.desktop";
    "application/xhtml+xml" = "firefox.desktop";
    "application/x-extension-xhtml" = "firefox.desktop";
    "application/x-extension-xht" = "firefox.desktop";
    "image/svg" = "firefox.desktop";
    "text/html" = "firefox.desktop";
  };

  # firefox persistent storage, maybe check if really everything in there is needed
  # note for preservation: if a subdirectory of .mozilla is persisted instead, correct file permissions for .mozilla must be set anyways with systemd-tmpfiles
  preservation.preserveAt.state-dir.directories = [ ".mozilla" ];
}
