{
  config,
  lib,
  pkgs,
  ...
}:
let
  cfg = config.programs.claude-code;
  json = (pkgs.formats.json { }).generate;

  # The manifest name becomes the MCP tool namespace
  # (`mcp__plugin_<name>_<server>__<tool>`), so it is kept short.
  pluginName = "local";
  pluginDir = ".claude/skills/nix-managed";
in
{
  options.programs.claude-code = {
    enable = lib.mkEnableOption "Claude Code";

    package = lib.mkOption {
      type = lib.types.nullOr lib.types.package;
      default = null;
      description = "The Claude Code package to install, or null to install none.";
    };

    settings = lib.mkOption {
      type = lib.types.attrs;
      default = { };
      description = "Written to {file}`~/.claude/settings.json`.";
    };

    skills = lib.mkOption {
      type = lib.types.attrsOf lib.types.lines;
      default = { };
      description = "Each entry becomes {file}`~/.claude/skills/<name>/SKILL.md`.";
    };

    lspServers = lib.mkOption {
      type = lib.types.attrs;
      default = { };
      description = ''
        Language servers offered to Claude Code, keyed by name. Delivered
        through the generated plugin rather than through settings.json.
      '';
    };

    mcpServers = lib.mkOption {
      type = lib.types.attrs;
      default = { };
      description = ''
        MCP servers offered to Claude Code, keyed by name. Values are passed
        through untouched, so `{file:...}` references resolve at load time and
        secrets stay out of the store.
      '';
    };
  };

  config = lib.mkIf cfg.enable {
    packages = lib.optional (cfg.package != null) cfg.package;

    files = {
      ".claude/settings.json" = {
        generator = json "claude-settings.json";
        value = {
          "$schema" = "https://json.schemastore.org/claude-code-settings.json";
        }
        // cfg.settings;
      };

      "${pluginDir}/.claude-plugin/plugin.json" = {
        generator = json "claude-plugin.json";
        value.name = pluginName;
      };
    }
    // lib.optionalAttrs (cfg.lspServers != { }) {
      "${pluginDir}/.lsp.json" = {
        generator = json "claude-lsp.json";
        value = cfg.lspServers;
      };
    }
    // lib.optionalAttrs (cfg.mcpServers != { }) {
      "${pluginDir}/.mcp.json" = {
        generator = json "claude-mcp.json";
        value.mcpServers = cfg.mcpServers;
      };
    }
    // lib.mapAttrs' (
      name: text: lib.nameValuePair ".claude/skills/${name}/SKILL.md" { inherit text; }
    ) cfg.skills;
  };
}
