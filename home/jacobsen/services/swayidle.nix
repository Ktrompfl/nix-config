{
  graphicalService,
  lib,
  pkgs,
  ...
}:
let
  lock = "${lib.getExe pkgs.swaylock} --daemonize";
in
{
  systemd.services.swayidle = graphicalService "session" {
    description = "Idle manager for Wayland";

    serviceConfig = {
      ExecStart = lib.concatStringsSep " " [
        (lib.getExe pkgs.swayidle)
        "-w"
        "lock '${lock}'"
        "before-sleep '${lock}'"
      ];
      Environment = [ "PATH=${lib.makeBinPath [ pkgs.bash ]}" ];
    };
  };
}
