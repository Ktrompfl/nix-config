{ config, lib, ... }:
let
  cfg = config.programs.gamemode;
in
{
  programs.gamemode = {
    enable = lib.mkDefault config.programs.steam.enable;
    settings.general.inhibit_screensaver = 0;
  };

  users.users.jacobsen.extraGroups = lib.optional cfg.enable "gamemode";
}
