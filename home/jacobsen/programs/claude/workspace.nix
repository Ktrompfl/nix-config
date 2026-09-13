{ config, osConfig, ... }:
let
  stateHome = "${osConfig.preservation.preserveAt.state-dir.persistentStoragePath}${config.directory}";

  workspace = "${stateHome}/.local/state/claude/workspace";
in
{
  files.".claude/settings.json".value = {
    env.TMPDIR = workspace;

    sandbox.filesystem = {
      denyRead = [ "/" ];

      allowRead = [
        "/nix"
        "/bin"
        "/etc"
        "/run"
        "/usr"
        workspace

        "${stateHome}/.config/git"
        "${stateHome}/.config/direnv"
        "~/.config/git"
        "~/.config/direnv"
        "~/.cargo"
      ];

      allowWrite = [
        workspace
        "~/.cache/nix"
        "~/.cargo"
        "~/.cache/uv"
        "~/.cache/ruff"
        "~/.julia"
      ];
    };

    permissions.additionalDirectories = [ workspace ];
  };

  files.".claude/CLAUDE.md".text = ''
    # Scratch files

    `/tmp` and `/var/tmp` are a 2 GB tmpfs shared with the root filesystem.
    Put build output, unpacked archives, checkouts, logs and anything else of
    unbounded size under `$TMPDIR` instead, one subdirectory per task; it is
    on disk and cleaned after 7 days. Never pass `/tmp` as an output
    directory.
  '';

  systemd.tmpfiles.rules = [ "d ${workspace} 0700 - - 7d" ];
}
