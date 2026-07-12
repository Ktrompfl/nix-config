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
      events = {
        lock = lock;
        before-sleep = lock;
      };
    };
}
