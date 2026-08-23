{ lib, pkgs }:
{
  Lua = {
    formatter = {
      external = {
        command = lib.getExe pkgs.stylua;
        arguments = [
          "--search-parent-directories"
          "--stdin-filepath"
          "{buffer_path}"
          "-"
        ];
      };
    };
  };
  Nix = {
    formatter.external = {
      command = lib.getExe pkgs.nixfmt;
    };
    language_servers = [
      "!nil"
      "nixd"
      "..."
    ];
    tab_size = 2;
  };
  Python = {
    code_actions_on_format = {
      "source.organizeImports.ruff" = true;
    };
    formatter = {
      language_server.name = "ruff";
    };
    language_servers = [
      "ruff"
      "ty"
      "!basedpyright"
      "!pylsp"
      "!pyright"
      "..."
    ];
  };
}
