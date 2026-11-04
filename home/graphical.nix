{ config, ... }:
{
  hjem.users.jacobsen.imports = [ ./graphical ];

  # chromium only reads policies from /etc
  programs.chromium = {
    enable = true;
    extraOpts.BrowserThemeColor = config.hjem.users.jacobsen.theme.colors.withHashtag.base00;
  };
}
