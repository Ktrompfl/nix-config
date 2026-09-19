{ config, osConfig, ... }:
let
  agentHome = ".local/state/claude/home";
  stateHome = "${osConfig.preservation.preserveAt.state-dir.persistentStoragePath}${config.directory}";

  workspace = "${stateHome}/.local/state/claude/workspace";
in
{
  files."${agentHome}/settings.json".value = {
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
        "${stateHome}/.local/share/cargo"
      ];

      # nix, uv and ruff all honour XDG_CACHE_HOME, which desktop/xdg.nix
      # points at ~/.local/cache; ~/.cache is not used and is not preserved.
      allowWrite = [
        workspace
        "~/.local/cache/nix"
        "~/.local/cache/uv"
        "~/.local/cache/ruff"
        "${stateHome}/.local/share/cargo"
        "~/.local/share/julia"
      ];
    };

    permissions.additionalDirectories = [ workspace ];
  };

  files."${agentHome}/CLAUDE.md".text = ''
    # Scratch files

    `/tmp` and `/var/tmp` are a 2 GB tmpfs shared with the root filesystem.
    Put build output, unpacked archives, checkouts, logs and anything else of
    unbounded size under `$TMPDIR` instead, one subdirectory per task; it is
    on disk and cleaned after 7 days. Never pass `/tmp` as an output
    directory.
  '';

  # Claude reads its own state from here rather than from ~/.claude.
  environment.sessionVariables.CLAUDE_CONFIG_DIR = "${config.directory}/${agentHome}";

  systemd.tmpfiles.rules = [ "d ${workspace} 0700 - - 7d" ];
}
