{
  inputs,
  lib,
  osConfig,
  pkgs,
  ...
}:
let
  llm-packages = inputs.llm-agents.packages.${pkgs.stdenv.hostPlatform.system};
  json = (pkgs.formats.json { }).generate;

  # The manifest name becomes the MCP tool namespace
  # (`mcp__plugin_<name>_<server>__<tool>`), so it is kept short.
  pluginName = "local";
  pluginDir = ".claude/skills/nix-managed";
in
{
  imports = [
    ./permissions.nix
    ./skills.nix
    ./workspace.nix
  ];

  packages = [
    llm-packages.claude-code
    llm-packages.ccusage

    # extra utilities
    pkgs.ast-grep
    pkgs.bubblewrap
    pkgs.socat
  ];

  files = {
    ".claude/settings.json" = {
      generator = json "claude-settings.json";
      value."$schema" = "https://json.schemastore.org/claude-code-settings.json";
    };

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
          command = lib.getExe pkgs.context7-mcp;
          args = [ ];
          env.CONTEXT7_API_KEY = "{file:${osConfig.sops.secrets."api-keys/context7".path}}";
        };
        nixos = {
          command = lib.getExe pkgs.mcp-nixos;
          args = [ ];
        };
      };
    };
  };

  preservation.preserveAt.state-dir.directories = [ ".claude" ];
}
