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
  stateHome = "${osConfig.preservation.preserveAt.state-dir.persistentStoragePath}${home}";
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

        # shared caches, so that builds are not cold in every session
        (try-readwrite "${home}/.local/cache")
        (try-readwrite "${home}/.local/share/julia")
        (try-readwrite "${stateHome}/.local/share/cargo")
        (try-readwrite "${stateHome}/.local/share/rustup")

        (try-readonly "${home}/.config/git")
        (try-readonly "${home}/.config/direnv")
        (try-readonly "${home}/.local/share/direnv") # the allow list
        (try-readonly "${stateHome}/.local/share/gurobi")
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
        (set-env "TMPDIR" "${home}/tmp")
      ]
      ++ lib.mapAttrsToList (name: value: set-env name (toString value)) (
        removeAttrs config.environment.sessionVariables [ "CLAUDE_CONFIG_DIR" ]
      )
      ++ lib.optional (zotero != null) (try-readonly zotero);
  };

  # ccusage and claudecode.nvim on the host find the jail's state through this.
  packages = [ llm-packages.ccusage ];
  environment.sessionVariables.CLAUDE_CONFIG_DIR = "${config.apps.claude.storage}/.claude";

  systemd.tmpfiles.rules = [ "e ${config.apps.claude.storage}/tmp - - - 7d" ];
}
