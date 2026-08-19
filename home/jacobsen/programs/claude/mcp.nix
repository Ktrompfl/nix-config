{
  lib,
  osConfig,
  pkgs,
  ...
}:
{
  programs.claude-code.mcpServers = {
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
}
