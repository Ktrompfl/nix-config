{ inputs, pkgs, ... }:
let
  llm-packages = inputs.llm-agents.packages.${pkgs.stdenv.hostPlatform.system};
in
{
  imports = [
    ./lsp.nix
    ./mcp.nix
    ./permissions
    ./skills.nix
    ./status.nix
  ];

  programs.claude-code = {
    enable = true;
    package = llm-packages.claude-code;
  };

  packages = [
    llm-packages.claude-desktop
    llm-packages.ccusage

    # extra utilities
    pkgs.ast-grep
  ];

  preservation.preserveAt.state-dir.directories = [
    ".claude"
    ".config/Claude"
  ];
}
