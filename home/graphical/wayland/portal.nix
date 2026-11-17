{ lib, pkgs, ... }:
{
  # upstream's unit without Requisite=graphical-session.target: a portal still
  # running after the session ends would keep the target "needed", so it never
  # stopped and the next session's services were not started again
  xdg.config.files."systemd/user/xdg-desktop-portal.service".text = ''
    [Unit]
    Description=Portal service
    PartOf=graphical-session.target
    After=graphical-session.target

    [Service]
    Type=dbus
    BusName=org.freedesktop.portal.Desktop
    ExecStart=${pkgs.xdg-desktop-portal}/libexec/xdg-desktop-portal
    Slice=session.slice
  '';

  xdg.config.files."xdg-desktop-portal-wlr/config".text = ''
    [screencast]
    chooser_type=dmenu
    chooser_cmd=${lib.getExe pkgs.fuzzel} --dmenu --prompt "share: "
  '';
}
