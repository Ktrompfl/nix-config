{
  inputs,
  pkgs,
  ...
}:
let
  llm-packages = inputs.llm-agents.packages.${pkgs.stdenv.hostPlatform.system};
in
{
  imports = [
    ./lsp.nix
    ./mcp.nix
    ./permissions.nix
    ./skills.nix
    ./status.nix
  ];

  home.packages = [
    llm-packages.claude-desktop
    llm-packages.ccusage

    # extra utilities
    pkgs.ast-grep
  ];

  # TODO: maybe install https://mcp-nixos.io/
  programs.claude-code = {
    enable = true;
    package = llm-packages.claude-code;
  };

  preservation.preserveAt.state-dir.directories = [
    ".claude"
    ".config/Claude"
  ];
}
