{ config, ... }:
{
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
      widget-overflow-fixed-list = [ ];
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
      vertical-tabs = [ ];
      PersonalToolbar = [ ];
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

  "font.name.serif.x-western" = config.theme.fonts.serif.name;
  "font.name.sans-serif.x-western" = config.theme.fonts.sansSerif.name;
  "font.name.monospace.x-western" = config.theme.fonts.monospace.name;
  "font.size.variable.x-western" = 16;
  "font.size.monospace.x-western" = 16;

  "reader.color_scheme" = "custom";
  "reader.custom_colors.background" = config.theme.colors.hex "background";
  "reader.custom_colors.foreground" = config.theme.colors.hex "foreground";
  "reader.custom_colors.selection-highlight" = config.theme.colors.hex "subtle";
  "reader.custom_colors.visited-links" = config.theme.colors.hex "keyword";
  "reader.custom_colors.unvisited-links" = config.theme.colors.hex "accent";
}
