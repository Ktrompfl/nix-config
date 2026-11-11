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

  # shared with the host, so builds stay warm
  shared = {
    ${config.xdg.cache.directory} = "rw";
    "${data}/julia" = "rw";
    "${data}/cargo" = "rw";
    "${data}/rustup" = "rw";
    "${data}/gurobi" = "ro";
  };

  # the host's session variables, minus those pointing into unshared storage
  below = directory: path: path == directory || lib.hasPrefix "${directory}/" path;
  private =
    value:
    lib.any (
      path:
      (below config.storage.state path || below config.storage.data path)
      && !lib.any (directory: below directory path) (lib.attrNames shared)
    ) (lib.splitString ":" value);
  variables = lib.filterAttrs (
    _: value: !private (toString value)
  ) config.environment.sessionVariables;
  zotero =
    if
      lib.any (entry: entry.directory == "Zotero") config.preservation.preserveAt.state-dir.directories
    then
      "${config.storage.state}/Zotero"
    else
      null;

  # becomes the MCP tool namespace: `mcp__plugin_<name>_<server>__<tool>`
  pluginName = "local";
  pluginDir = ".claude/skills/nix-managed";

  context7 = pkgs.writeShellScriptBin "mcp-context7" ''
    export CONTEXT7_API_KEY="$(cat ${osConfig.sops.secrets."api-keys/context7".path})"
    exec ${lib.getExe pkgs.context7-mcp} "$@"
  '';
in
{
  imports = [ ./skills.nix ];

  # only ever runs in this jail, so it may act without asking
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

          # direnv environment for Bash commands, re-evaluated on every call
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

      # language and MCP servers come through a generated plugin
      "${pluginDir}/.claude-plugin/plugin.json" = {
        generator = json "claude-plugin.json";
        value.name = pluginName;
      };
      "${pluginDir}/.lsp.json" = {
        generator = json "claude-lsp.json";
        value = (import ../lsp.nix { inherit config lib pkgs; }).claude;
      };
      "${pluginDir}/.mcp.json" = {
        generator = json "claude-mcp.json";
        value.mcpServers = {
          context7 = {
            command = lib.getExe context7;
            args = [ ];
          };
          # FIXME: restore once fastmcp-slim supports mcp >= 2 (marked broken in nixpkgs)
          # nixos = {
          #   command = lib.getExe pkgs.mcp-nixos;
          #   args = [ ];
          # };
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

        # keeps SIGWINCH on resize; TIOCSTI is disabled by nix-mineral anyway
        no-new-session

        # mount-cwd from $HOME would cover the private home
        (add-runtime ''
          if [ "$PWD" = "$HOME" ]; then
            echo "claude: refusing to expose the whole home directory" >&2
            exit 1
          fi
        '')
        mount-cwd
        # the logical cwd; the physical one is a /persist path
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

        # $EDITOR is the host's Zed, which reaches its CLI through a socket in
        # $TMPDIR, so that has to be a path the host also has
        (try-ro-bind "${data}/zed" "${home}/.local/share/zed")
        (readwrite "${storage}/tmp")
        (set-env "TMPDIR" "${storage}/tmp")
      ]
      ++ lib.mapAttrsToList (
        path: mode: if mode == "rw" then try-readwrite path else try-readonly path
      ) shared
      ++ lib.mapAttrsToList (name: value: set-env name (toString value)) variables
      ++ lib.optional (zotero != null) (try-readonly zotero);
  };

  # for ccusage and claudecode.nvim on the host
  packages = [ llm-packages.ccusage ];
  environment.sessionVariables.CLAUDE_CONFIG_DIR = "${storage}/.claude";

  systemd.tmpfiles.rules = [ "e ${storage}/tmp - - - 7d" ];
}
