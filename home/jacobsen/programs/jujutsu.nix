{ config, ... }:
{
  programs.jujutsu = {
    enable = true;
    settings = {
      inherit (config.programs.git.settings.user) email name;
    };
  };
}
