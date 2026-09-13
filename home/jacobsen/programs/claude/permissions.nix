{ config, osConfig, ... }:
let
  credentials = "${osConfig.preservation.preserveAt.state-dir.persistentStoragePath}${config.directory}/.claude/.credentials.json";
in
{
  files.".claude/settings.json".value = {
    sandbox = {
      enabled = true;
      autoAllowBashIfSandboxed = true;

      excludedCommands = [
        "sudo *"
        "nixos-rebuild *"
        "nh *"
        "systemctl *"
        "wl-copy *"
        "wl-paste *"
      ];

      network = {
        allowedDomains = [
          "cache.nixos.org"
          "cache.numtide.com"
          "channels.nixos.org"
          "nixos.org"
          "search.nixos.org"
          "*.cachix.org"

          "github.com"
          "api.github.com"
          "codeload.github.com"
          "*.githubusercontent.com"

          "crates.io"
          "*.crates.io"
          "docs.rs"
          "doc.rust-lang.org"
          "registry.npmjs.org"
          "pypi.org"
          "files.pythonhosted.org"

          "context7.com"
          "*.context7.com"
          "devenv.sh"
        ];

        allowAllUnixSockets = true;
      };
    };

    permissions = {
      blockReadsOutsideWorkingDirectories = true;

      defaultMode = "acceptEdits";

      allow = [
        "WebSearch"
        "WebFetch(domain:github.com)"
        "WebFetch(domain:raw.githubusercontent.com)"
        "WebFetch(domain:docs.rs)"
        "WebFetch(domain:devenv.sh)"

        "mcp__plugin_local_context7__*"
        "mcp__plugin_local_nixos__*"
      ];

      deny = [
        "Read(/${config.directory}/.ssh/**)"
        "Read(/${credentials})"
        "Read(//run/secrets.d/**)"
        "Read(//persist/sops/**)"
      ];
    };
  };
}
