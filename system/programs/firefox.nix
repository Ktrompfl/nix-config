{ pkgs, ... }:
{
  programs.firefox = {
    enable = true;
    nativeMessagingHosts.packages = [ pkgs.tridactyl-native ];

    policies = {
      # debloat
      AppAutoUpdate = false;
      CaptivePortal = false;
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
      DNSOverHTTPS = {
        Enabled = true;
        ProviderURL = "https://mozilla.cloudflare-dns.com/dns-query";
        Locked = false;
        Fallback = false;
      };
    };
  };
}
