{
  lib,
  pkgs,
  ...
}:
{
  services.swayidle =
    let
      lock = "${lib.getExe pkgs.swaylock} --daemonize";
      display = status: "swaymsg 'output * power ${status}'";
    in
    {
      enable = true;
      timeouts = [
        {
          timeout = 300;
          command = lock;
        }
        {
          timeout = 330;
          command = display "off";
          resumeCommand = display "on";
        }
        {
          timeout = 900;
          command = "systemctl suspend";
        }
      ];
      events = [
        {
          event = "before-sleep";
          # adding duplicated entries for the same event may not work
          command = (display "off") + "; " + lock;
        }
        {
          event = "after-resume";
          command = display "on";
        }
        {
          event = "lock";
          command = (display "off") + "; " + lock;
        }
        {
          event = "unlock";
          command = display "on";
        }
      ];
    };
}
