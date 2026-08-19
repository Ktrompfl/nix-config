{ config, ... }:
{
  # policies only; the browser itself is a user package
  programs.chromium = {
    enable = true;
    extraOpts.BrowserThemeColor = config.theme.colors.withHashtag.base00;
  };
}
