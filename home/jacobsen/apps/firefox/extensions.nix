{
  config,
  lib,
  pkgs,
  ...
}:
let
  profile = ".mozilla/firefox/default";

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

  # `permissions` restates what each addon's manifest asks for. Firefox grants
  # those implicitly for extensions dropped into the profile's `extensions`
  # directory, so there is never a prompt, and an update that widens them
  # would otherwise land unnoticed. The assertions below compare each list
  # against `meta.mozPermissions`, which is read from the xpi itself.
  #
  # `storage` seeds an addon's own configuration. Reaching it at all depends
  # on the flat JSON storage backend, which ./preferences.nix pins.
  addons = with pkgs.nur.repos.rycee.firefox-addons; [
    {
      # applies the theme palette to the browser chrome
      package = firefox-color;

      permissions = [
        "theme"
        "storage"
        "tabs"
        "https://color.firefox.com/*"
      ];

      storage = firefoxColorTheme;
    }
    {
      # password manager
      package = bitwarden;

      permissions = [
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
    }
    {
      # dark mode for every website
      package = darkreader;

      permissions = [
        "alarms"
        "contextMenus"
        "storage"
        "tabs"
        "theme"
        "<all_urls>"
      ];

      storage = {
        theme = with config.theme.colors.withHashtag; {
          fontFamily = config.theme.fonts.sansSerif.name;
          lightSchemeBackgroundColor = base00;
          darkSchemeBackgroundColor = base00;
          lightSchemeTextColor = base05;
          darkSchemeTextColor = base05;
          selectionColor = base0D;
        };
      };
    }
    {
      # automatically handle gdpr consent forms
      package = consent-o-matic;

      permissions = [
        "activeTab"
        "tabs"
        "storage"
        "<all_urls>"
      ];
    }
    {
      # ad blocker
      package = ublock-origin;

      permissions = [
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
    }
    {
      # vim bindings for the browser
      package = tridactyl;

      permissions = [
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
    }
    {
      # save references and PDFs to Zotero
      package = zotero-connector.override (old: {
        # The nur expression passes mozPermissions as a builder argument
        # and buildFirefoxXpiAddon does not copy it into `meta` for this
        # one, leaving the check below with nothing to compare against.
        meta = old.meta // {
          inherit (old) mozPermissions;
        };
      });

      permissions = [
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
    }
  ];

  manifestOf = addon: addon.package.meta.mozPermissions or null;
  ordered = lib.sort lib.lessThan;
in
{
  assertions = map (
    addon:
    let
      declared = ordered addon.permissions;
      manifest = manifestOf addon;
      id = addon.package.addonId;
    in
    {
      assertion = manifest != null && declared == ordered manifest;
      message =
        if manifest == null then
          "firefox: ${id} exposes no meta.mozPermissions, so its permissions cannot be checked"
        else
          ''
            firefox: ${id} no longer asks for the permissions declared for it.
              newly requested: ${toString (lib.subtractLists declared (ordered manifest))}
              no longer asked: ${toString (lib.subtractLists (ordered manifest) declared)}
          '';
    }
  ) addons;

  apps.firefox.files =
    # The xpis are never rewritten and are large enough that seeding a copy of
    # each one before every launch would be noticeable.
    lib.listToAttrs (
      map (addon: {
        name = "${profile}/extensions/${addon.package.addonId}.xpi";
        value = {
          source = "${addon.package}/share/mozilla/extensions/{ec8030f7-c20a-464f-9b0e-13a3a9e97384}/${addon.package.addonId}.xpi";
          mutable = false;
        };
      }) addons
    )
    // lib.listToAttrs (
      map (addon: {
        name = "${profile}/browser-extension-data/${addon.package.addonId}/storage.js";
        value = {
          generator = (pkgs.formats.json { }).generate "${addon.package.addonId}-storage.js";
          value = addon.storage;
        };
      }) (lib.filter (addon: addon ? storage) addons)
    );
}
