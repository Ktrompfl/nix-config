{
  config,
  lib,
  pkgs,
  ...
}:
{
  sops.secrets."api-keys/context7" = { };

  programs = {
    mcp = {
      enable = true;
      servers = {
        context7 = {
          enabled = true;
          command = lib.getExe pkgs.context7-mcp;
          args = [ ];
          env = {
            CONTEXT7_API_KEY.file = config.sops.secrets."api-keys/context7".path;
          };
        };
        nixos = {
          enabled = true;
          command = lib.getExe pkgs.mcp-nixos;
          args = [ ];
        };
      };
    };

    # agent / editor integration
    claude-code.enableMcpIntegration = true;
    zed-editor.enableMcpIntegration = true;
  };
}
