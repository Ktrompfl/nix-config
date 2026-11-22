{ lib, pkgs, ... }:
{
  environment.systemPackages = with pkgs; [
    getaway
    xwayland-satellite
  ];

  security = {
    polkit.enable = true;
    pam.services.swaylock = { };
    wrappers.getaway = {
      owner = "root";
      group = "root";
      permissions = "a+rx";
      source = lib.getExe pkgs.getaway;
      capabilities = "cap_sys_nice+p";
    };
  };

  programs.dconf.enable = true;

  xdg.portal = {
    enable = true;
    # mkDefault like the jay module, so both lists merge instead of this one
    # replacing jay-portals.conf
    configPackages = lib.mkDefault [ pkgs.getaway ];
    extraPortals = with pkgs; [
      xdg-desktop-portal-gtk
      xdg-desktop-portal-wlr
    ];
  };

  services.displayManager.sessionPackages = [ pkgs.getaway ];
}
