{ pkgs, ... }:
{
  packages = with pkgs; [
    runapp
    jay-screenshot
    playerctl
    wl-mirror
  ];

  xdg.config.files."jay/config.so".source = "${pkgs.jay-config-lib}/lib/config.so";
}
