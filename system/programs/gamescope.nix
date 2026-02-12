{ config, lib, ... }:
{
  programs.gamescope = {
    enable = lib.mkDefault config.programs.steam.enable;
  };
}
