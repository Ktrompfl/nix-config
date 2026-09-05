{ config, pkgs, ... }:
let
  firefoxColorTheme =
    let
      mkColor = config.theme.colors.channels;
    in
    {
      firstRunDone = true;
      theme = {
        title = "Tinted";
        images.additional_backgrounds = [ "./bg-000.svg" ];
        colors = {
          toolbar = mkColor "base00";
          toolbar_text = mkColor "base05";
          frame = mkColor "base01";
          tab_background_text = mkColor "base05";
          toolbar_field = mkColor "base02";
          toolbar_field_text = mkColor "base05";
          tab_line = mkColor "base0D";
          popup = mkColor "base00";
          popup_text = mkColor "base05";
          button_background_active = mkColor "base04";
          frame_inactive = mkColor "base00";
          icons_attention = mkColor "base0D";
          icons = mkColor "base05";
          ntp_background = mkColor "base00";
          ntp_text = mkColor "base05";
          popup_border = mkColor "base0D";
          popup_highlight_text = mkColor "base05";
          popup_highlight = mkColor "base04";
          sidebar_border = mkColor "base0D";
          sidebar_highlight_text = mkColor "base05";
          sidebar_highlight = mkColor "base0D";
          sidebar_text = mkColor "base05";
          sidebar = mkColor "base00";
          tab_background_separator = mkColor "base0D";
          tab_loading = mkColor "base05";
          tab_selected = mkColor "base00";
          tab_text = mkColor "base05";
          toolbar_bottom_separator = mkColor "base00";
          toolbar_field_border_focus = mkColor "base0D";
          toolbar_field_border = mkColor "base00";
          toolbar_field_focus = mkColor "base00";
          toolbar_field_highlight_text = mkColor "base00";
          toolbar_field_highlight = mkColor "base0D";
          toolbar_field_separator = mkColor "base0D";
          toolbar_vertical_separator = mkColor "base0D";
        };
      };
    };

  # settings shared by every profile
in
{
  force = true;
  exactPermissions = true;
  exhaustivePermissions = true;

  packages = with pkgs.nur.repos.rycee.firefox-addons; [
    firefox-color # applies the theme palette to the browser chrome
    bitwarden # password manager
    darkreader # dark mode for every website
    consent-o-matic # automatically handle gdpr consent forms
    ublock-origin # ad blocker
    tridactyl
    # save references/PDFs to Zotero
    # the nur package declares mozPermissions at the top level, but
    # buildMozillaXpiAddon only propagates `meta`, so lift them into it to
    # make them visible to the permission check
    (zotero-connector.override (old: {
      meta = old.meta // {
        inherit (old) mozPermissions;
      };
    }))
  ];

  settings = {
    # firefox color (installed below, themed from ./theme)
    "FirefoxColor@mozilla.com" = {
      permissions = [
        "theme"
        "storage"
        "tabs"
        "https://color.firefox.com/*"
      ];
      settings = firefoxColorTheme;
    };
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
    "addon@darkreader.org".settings.theme = with config.theme.colors.withHashtag; {
      fontFamily = config.theme.fonts.sansSerif.name;
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
    # zotero connector
    "zotero@chnm.gmu.edu".permissions = [
      "http://*/*"
      "https://*/*"
      "tabs"
      "contextMenus"
      "cookies"
      "storage"
      "scripting"
      "webRequest"
      "webRequestBlocking"
      "webNavigation"
      "declarativeNetRequest"
      "management"
      "clipboardWrite"
    ];
  };
}
