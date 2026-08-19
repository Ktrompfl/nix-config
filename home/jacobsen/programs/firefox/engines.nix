{ pkgs, ... }:
{
  google.metaData.alias = "@g";
  wikipedia.metaData.alias = "@wiki";
  bing.metaData.hidden = true;
  duckduckgo.metaData.hidden = true;

  "Nix Packages" = {
    urls = [
      {
        template = "https://search.nixos.org/packages";
        params = [
          {
            name = "channel";
            value = "unstable";
          }
          {
            name = "query";
            value = "{searchTerms}";
          }
        ];
      }
    ];
    icon = "${pkgs.nixos-icons}/share/icons/hicolor/scalable/apps/nix-snowflake.svg";
    definedAliases = [ "@np" ];
  };

  "NixOS Wiki" = {
    urls = [
      {
        template = "https://wiki.nixos.org/w/index.php";
        params = [
          {
            name = "search";
            value = "{searchTerms}";
          }
        ];
      }
    ];
    icon = "${pkgs.nixos-icons}/share/icons/hicolor/scalable/apps/nix-snowflake.svg";
    updateInterval = 24 * 60 * 60 * 1000; # every day
    definedAliases = [ "@nw" ];
  };

  "Nix Options" = {
    definedAliases = [ "@no" ];
    icon = "${pkgs.nixos-icons}/share/icons/hicolor/scalable/apps/nix-snowflake.svg";
    urls = [
      {
        template = "https://search.nixos.org/options";
        params = [
          {
            name = "channel";
            value = "unstable";
          }
          {
            name = "query";
            value = "{searchTerms}";
          }
        ];
      }
    ];
  };

  "Home Manager Options" = {
    definedAliases = [ "@hm" ];
    icon = "${pkgs.nixos-icons}/share/icons/hicolor/scalable/apps/nix-snowflake.svg";
    urls = [
      { template = "https://home-manager-options.extranix.com/?query={searchTerms}&release=master"; }
    ];
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

  "Youtube" = {
    definedAliases = [ "@yt" ];
    urls = [ { template = "https://youtube.com/search?q={searchTerms}"; } ];
    iconMapObj."16" = "https://www.youtube.com/s/desktop/606e092f/img/logos/favicon.ico";
  };
}
