# The session command greetd starts.
{
  lib,
  systemd,
  writeShellApplication,
}:
writeShellApplication {
  name = "jay-session";

  runtimeInputs = [ systemd ];

  text = ''
    systemctl --user import-environment XDG_SESSION_ID PATH
    exec systemctl --user restart --wait jay.service
  '';

  meta = {
    description = "Runs jay as a user service for the duration of the login session";
    platforms = lib.platforms.linux;
  };
}
