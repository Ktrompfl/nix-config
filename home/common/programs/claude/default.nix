{
  config,
  inputs,
  lib,
  osConfig,
  pkgs,
  ...
}:
let
  llm-packages = inputs.llm-agents.packages.${pkgs.stdenv.hostPlatform.system};
  json = (pkgs.formats.json { }).generate;

  home = config.directory;
  storage = config.apps.claude.storage;
  data = config.xdg.data.directory;

  # State shared with the host, so that builds are not cold in every session.
  shared = {
    ${config.xdg.cache.directory} = "rw";
    "${data}/julia" = "rw";
    "${data}/cargo" = "rw";
    "${data}/rustup" = "rw";
    "${data}/gurobi" = "ro";
  };

  # The session variables come in too, bar those that name the host's storage
  # outside of what is shared: the data and state homes among them, so that
  # whatever else runs in here keeps its state in the private home.
  below = directory: path: path == directory || lib.hasPrefix "${directory}/" path;
  private =
    value:
    lib.any (
      path:
      (below config.storage.state path || below config.storage.data path)
      && !lib.any (directory: below directory path) (lib.attrNames shared)
    ) (lib.splitString ":" value);
  variables = lib.filterAttrs (_: value: !private (toString value)) config.environment.sessionVariables;
  zotero = if config.apps ? zotero then "${config.apps.zotero.storage}/Zotero" else null;

  # The manifest name becomes the MCP tool namespace
  # (`mcp__plugin_<name>_<server>__<tool>`), so it is kept short.
  pluginName = "local";
  pluginDir = ".claude/skills/nix-managed";

  context7 = pkgs.writeShellScriptBin "mcp-context7" ''
    export CONTEXT7_API_KEY="$(cat ${osConfig.sops.secrets."api-keys/context7".path})"
    exec ${lib.getExe pkgs.context7-mcp} "$@"
  '';
in
{
  imports = [ ./skills.nix ];

  # Claude only ever runs in this jail, so it may do anything in there without
  # asking. Its login, history and config live in the jail's private home.
  apps.claude = {
    package = llm-packages.claude-code;

    files = {
      ".claude/settings.json" = {
        generator = json "claude-settings.json";
        value = {
          "$schema" = "https://json.schemastore.org/claude-code-settings.json";

          permissions.defaultMode = "bypassPermissions";
          skipDangerousModePermissionPrompt = true;

          # the terminal's sixteen colours, which follow the system scheme
          theme = "${config.theme.colors.meta.variant or "dark"}-ansi";

          # Bash commands get the project's direnv environment, re-evaluated
          # before each one so that it follows `cd`. An .envrc that was never
          # allowed on the host stays inert, since the allow list is read-only
          # in here.
          hooks.SessionStart = [
            {
              hooks = [
                {
                  type = "command";
                  command = ''echo 'eval "$(direnv export bash 2>/dev/null)"' >> "$CLAUDE_ENV_FILE"'';
                }
              ];
            }
          ];
        };
      };

      ".claude/CLAUDE.md".text = ''
        # Environment

        - You run in a bubblewrap jail. Host paths visible: ~/Repositories and
          /persist/nixos (rw), the start directory, build caches, /nix (builds
          via the host daemon). Commands describe the jail, not the host; for
          host facts read /persist/nixos or ask.
        - /persist/nixos is the host's NixOS config: edit and build on request,
          never deploy.
        - Project tools come from direnv; if missing, ask the user to run
          `direnv allow` on the host.
      ''
      + lib.optionalString (zotero != null) ''
        - Zotero: ${zotero} (ro; PDFs in storage/, zotero.sqlite locked while
          Zotero runs); local API at http://localhost:23119/api/.
      ''
      + ''
        - Missing path, device, credential or network access: stop and name
          what to add to apps.claude in
          /persist/nixos/home/common/programs/claude/default.nix; don't work
          around it.
        - /tmp is small and in memory; put large output under `$TMPDIR` (on
          disk, one subdirectory per task, cleaned after 7 days).
      '';

      # Language servers and MCP servers are delivered through a generated
      # plugin rather than through settings.json.
      "${pluginDir}/.claude-plugin/plugin.json" = {
        generator = json "claude-plugin.json";
        value.name = pluginName;
      };
      "${pluginDir}/.lsp.json" = {
        generator = json "claude-lsp.json";
        value = (import ../lsp.nix { inherit lib pkgs; }).claude;
      };
      "${pluginDir}/.mcp.json" = {
        generator = json "claude-mcp.json";
        value.mcpServers = {
          context7 = {
            command = lib.getExe context7;
            args = [ ];
          };
          nixos = {
            command = lib.getExe pkgs.mcp-nixos;
            args = [ ];
          };
        };
      };
    };

    directories = [ "tmp" ];
    access.Repositories = "rw";

    jail.permissions =
      c:
      with c;
      [
        network
        gpu
        (add-pkg-deps [ pkgs.ast-grep ])

        # Without a session of its own claude stays in the terminal's
        # foreground process group, and so gets the SIGWINCH that tells it the
        # terminal was resized. What --new-session guards against, typing into
        # the terminal through TIOCSTI, is refused anyway while
        # dev.tty.legacy_tiocsti is 0, which nix-mineral sets.
        no-new-session

        # Started from $HOME, mount-cwd would put the real home over the
        # private one.
        (add-runtime ''
          if [ "$PWD" = "$HOME" ]; then
            echo "claude: refusing to expose the whole home directory" >&2
            exit 1
          fi
        '')
        mount-cwd
        # bwrap would otherwise start in the physical cwd, which below
        # ~/Repositories is a /persist path the jail does not have
        (unsafe-add-raw-args "--chdir \"$PWD\"")

        (readwrite "/persist/nixos")

        (try-readonly "${home}/.config/git")
        (try-readonly "${home}/.config/direnv")

        # direnv looks for its allow list in the private data home
        (try-ro-bind "${data}/direnv" "${home}/.local/share/direnv")
        (readonly osConfig.sops.secrets."api-keys/context7".path)

        # nix goes through the host daemon
        (readonly "/nix/var/nix/daemon-socket")
        (readonly "/etc/nix")
        (readonly "/etc/static")
        (set-env "NIX_REMOTE" "daemon")

        # the host's userland, rather than a hand-picked subset of it
        (readonly "/run/current-system")
        (readonly "/etc/profiles/per-user/${config.user}")
        (add-path "/run/current-system/sw/bin")
        (add-path "/etc/profiles/per-user/${config.user}/bin")
        (try-readonly "/usr/bin/env")

        # unpatched binaries: pip wheels, julia artifacts, prebuilt tools
        (try-readonly "/lib64")
        (try-fwd-env "NIX_LD")
        (try-fwd-env "NIX_LD_LIBRARY_PATH")

        # ROCm, on top of what `gpu` binds
        (unsafe-add-raw-args "--dev-bind-try /dev/kfd /dev/kfd")
        (try-readonly "/sys/class/kfd")
        (try-readonly "/sys/devices/virtual/kfd")
        (try-readonly "/opt/rocm")

        (try-fwd-env "COLORTERM")

        # $EDITOR, which comes in with the session variables below, is the
        # host's Zed. Its CLI hands a path to the running instance over the
        # socket in Zed's data directory, and that instance then connects back
        # to a socket the CLI makes under $TMPDIR and opens the file, which
        # also lives there. So $TMPDIR is the same directory as ~/tmp in here,
        # but at the path the host has for it.
        (try-ro-bind "${data}/zed" "${home}/.local/share/zed")
        (readwrite "${storage}/tmp")
        (set-env "TMPDIR" "${storage}/tmp")
      ]
      # bound where the host has them, since that is what the variables say
      ++ lib.mapAttrsToList (path: mode: if mode == "rw" then try-readwrite path else try-readonly path) shared
      ++ lib.mapAttrsToList (name: value: set-env name (toString value)) variables
      ++ lib.optional (zotero != null) (try-readonly zotero);
  };

  # ccusage and claudecode.nvim on the host find the jail's state through this.
  packages = [ llm-packages.ccusage ];
  environment.sessionVariables.CLAUDE_CONFIG_DIR = "${storage}/.claude";

  systemd.tmpfiles.rules = [ "e ${storage}/tmp - - - 7d" ];
}
