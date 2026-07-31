{
  lib,
  pkgs,
  ...
}:
let
  inherit (lib)
    getExe
    getExe'
    genAttrs
    ;

  toLang = lang: exts: genAttrs exts (_: lang);
in
{
  programs.claude-code.lspServers = {
    bashls = {
      command = getExe pkgs.bash-language-server;
      args = [ "start" ];
      extensionToLanguage = toLang "shellscript" [
        ".sh"
        ".bash"
      ];
    };

    basedpyright = {
      command = getExe' pkgs.basedpyright "basedpyright-langserver";
      args = [ "--stdio" ];
      extensionToLanguage = toLang "python" [
        ".py"
        ".pyi"
        ".pyw"
      ];
    };

    clangd = {
      command = getExe' pkgs.clang-tools "clangd";
      args = [
        "--background-index"
        "--clang-tidy"
        "--header-insertion=iwyu"
        "--completion-style=detailed"
        "--function-arg-placeholders"
        "--fallback-style=llvm"
      ];
      initializationOptions = {
        usePlaceholders = true;
        completeUnimported = true;
        clangdFileStatus = true;
      };
      extensionToLanguage =
        (toLang "c" [
          ".c"
          ".h"
        ])
        // (toLang "cpp" [
          ".cpp"
          ".cc"
          ".cxx"
          ".c++"
          ".hpp"
          ".hh"
          ".hxx"
          ".h++"
        ]);
    };

    cmake = {
      command = getExe pkgs.cmake-language-server;
      extensionToLanguage = toLang "cmake" [ ".cmake" ];
    };

    cssls = {
      command = getExe' pkgs.vscode-langservers-extracted "vscode-css-language-server";
      args = [ "--stdio" ];
      extensionToLanguage = {
        ".css" = "css";
        ".scss" = "scss";
        ".less" = "less";
      };
    };

    emmylua-ls = {
      command = getExe pkgs.emmylua-ls;
      extensionToLanguage = toLang "lua" [ ".lua" ];
    };

    fish-lsp = {
      command = getExe pkgs.fish-lsp;
      extensionToLanguage = toLang "fish" [ ".fish" ];
    };

    html = {
      command = getExe' pkgs.vscode-langservers-extracted "vscode-html-language-server";
      args = [ "--stdio" ];
      extensionToLanguage = toLang "html" [
        ".html"
        ".htm"
      ];
    };

    jsonls = {
      command = getExe' pkgs.vscode-langservers-extracted "vscode-json-language-server";
      args = [ "--stdio" ];
      extensionToLanguage = {
        ".json" = "json";
        ".jsonc" = "jsonc";
      };
    };

    julia = {
      command = lib.getExe pkgs.julia-bin;
      args = [
        "--startup-file=no"
        "--history-file=no"
        "--quiet"
        "--project=@languageserver"
        "-e"
        "using LanguageServer; runserver()"
      ];
      extensionToLanguage = toLang "julia" [ ".jl" ];
      startupTimeout = 90000;
    };

    latex = {
      command = lib.getExe pkgs.texlab;
      extensionToLanguage = {
        ".bib" = "bibtex";
        ".cls" = "latex";
        ".sty" = "latex";
        ".tex" = "latex";
      };
      transport = "stdio";
    };

    marksman = {
      command = getExe pkgs.marksman;
      extensionToLanguage = toLang "markdown" [
        ".md"
        ".markdown"
        ".mdx"
      ];
    };

    nixd = {
      command = getExe pkgs.nixd;
      extensionToLanguage = toLang "nix" [ ".nix" ];
    };

    ruff = {
      command = getExe pkgs.ruff;
      args = [ "server" ];
      extensionToLanguage = toLang "python" [
        ".py"
        ".pyi"
        ".pyw"
      ];
    };

    rust-analyzer = {
      command = getExe pkgs.rust-analyzer;
      extensionToLanguage = toLang "rust" [ ".rs" ];
    };

    taplo = {
      command = getExe pkgs.taplo;
      args = [
        "lsp"
        "stdio"
      ];
      extensionToLanguage = toLang "toml" [ ".toml" ];
    };

    typst = {
      command = lib.getExe pkgs.tinymist;
      extensionToLanguage = toLang "typst" [ ".typ" ];
    };

    yamlls = {
      command = getExe pkgs.yaml-language-server;
      args = [ "--stdio" ];
      extensionToLanguage = toLang "yaml" [
        ".yaml"
        ".yml"
      ];
    };
  };
}
