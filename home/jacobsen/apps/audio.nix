{ pkgs, ... }:
{
  apps = {
    pwvucontrol = {
      package = pkgs.pwvucontrol;
      appId = "com.saivert.pwvucontrol";

      jail.permissions = c: with c; [ desktop ];
    };

    # mixer for the focusrite interface
    alsa-scarlett-gui = {
      package = pkgs.alsa-scarlett-gui;
      appId = "vu.b4.alsa-scarlett-gui";

      jail.permissions =
        c: with c; [
          desktop
          sound
        ];
    };
  };
}
