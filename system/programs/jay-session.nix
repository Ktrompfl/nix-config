{ lib, pkgs, ... }:
{
  # jay must be resolved from path to find the security wrapper with realtime scheduling
  environment.systemPackages = [
    (pkgs.writeShellScriptBin "jay-session" ''
      cleanup() {
        ${lib.getExe' pkgs.systemd "systemctl"} --user stop graphical-session.target || true
      }
      trap cleanup EXIT

      exec jay run "$@"
    '')
  ];
}
