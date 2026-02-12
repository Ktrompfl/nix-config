{ pkgs, ... }:
{
  home.packages = [ pkgs.moonlight-qt ]; # remote play

  preservation.preserveAt.state-dir.directories = [
    ".config/Moonlight Game Streaming Project/Moonlight.conf"
  ];
}
