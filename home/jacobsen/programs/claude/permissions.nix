{ config, ... }:
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

      credentials.files = [
        {
          path = "~/.ssh";
          mode = "deny";
        }
        {
          path = "~/.claude/.credentials.json";
          mode = "deny";
        }
        {
          path = "/run/secrets";
          mode = "deny";
        }
      ];

      network = {
        allowedDomains = [
          "cache.nixos.org"
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

          "devenv.sh"
        ];

        allowUnixSockets = [ "/nix/var/nix/daemon-socket/socket" ];
      };
    };

    permissions = {
      allow = [
        "Read(//nix/store/**)"
        "Read(/${config.directory}/.claude/**)"

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
        "Read(/${config.directory}/.claude/.credentials.json)"
        "Read(//run/secrets/**)"
      ];
    };
  };
}
