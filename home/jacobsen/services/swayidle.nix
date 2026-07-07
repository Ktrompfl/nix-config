{
  lib,
  pkgs,
  ...
}:
{
  services.swayidle =
    let
      lock = "${lib.getExe pkgs.swaylock} --daemonize";
    in
    {
      enable = true;
      events = [
        {
          event = "before-sleep";
          command = lock;
        }
        {
          event = "lock";
          command = lock;
        }
      ];
    };
}
