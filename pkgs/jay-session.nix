# The session command greetd starts, see ../system/programs/jay-session.nix.
{
  lib,
  systemd,
  writeShellApplication,
}:
writeShellApplication {
  name = "jay-session";

  # `jay` is deliberately not a runtime input: it has to be resolved from
  # `PATH` so that the security wrapper with realtime scheduling is the one
  # that runs, rather than the plain binary in the store.
  runtimeInputs = [ systemd ];

  text = ''
    while IFS= read -r assignment; do
      if [ -n "$assignment" ]; then
        export "''${assignment?}"
      fi
    done < <(${systemd}/lib/systemd/user-environment-generators/30-systemd-environment-d-generator)

    cleanup() {
      systemctl --user stop graphical-session.target || true
    }
    trap cleanup EXIT

    exec jay run "$@"
  '';

  meta = {
    description = "Starts the jay compositor and tears its session down again";
    platforms = lib.platforms.linux;
  };
}
