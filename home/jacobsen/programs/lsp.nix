{ lib, pkgs }:
let
  inherit (lib)
    getExe
    getExe'
    genAttrs
    mapAttrs
    optionalAttrs
    recursiveUpdate
    ;

  toLang = lang: exts: genAttrs exts (_: lang);

  bash-language-server = {
    command = getExe pkgs.bash-language-server;
    args = [ "start" ];
    languages = toLang "shellscript" [
      ".sh"
      ".bash"
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
    languages =
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

  cmake-language-server = {
    command = getExe pkgs.cmake-language-server;
    languages = toLang "cmake" [ ".cmake" ];
  };

  fish-lsp = {
    command = getExe pkgs.fish-lsp;
    languages = toLang "fish" [ ".fish" ];
  };

  harper-ls = {
    command = getExe' pkgs.harper "harper-ls";
    args = [ "--stdio" ];
    settings.harper-ls.dialect = "British";
  };

  jetls = {
    settings = {
      code_lens.references = true;
      formatter.custom = {
        executable = getExe pkgs.runic;
        executable_range = getExe pkgs.runic;
      };
    };
  };

  json-language-server = {
    command = getExe' pkgs.vscode-langservers-extracted "vscode-json-language-server";
    args = [ "--stdio" ];
    languages = {
      ".json" = "json";
      ".jsonc" = "jsonc";
    };
  };

  julia-lsp = {
    command = getExe (pkgs.julia.withPackages [ "LanguageServer" ]);
    args = [
      "--startup-file=no"
      "--history-file=no"
      "--quiet"
      "--project=@languageserver"
      "-e"
      "using LanguageServer; runserver()"
    ];
    languages = toLang "julia" [ ".jl" ];
    startupTimeout = 90000;
  };

  lua-language-server = {
    command = getExe pkgs.lua-language-server;
    languages = toLang "lua" [ ".lua" ];
  };

  marksman = {
    command = getExe pkgs.marksman;
    languages = toLang "markdown" [
      ".md"
      ".markdown"
      ".mdx"
    ];
  };

  nixd = {
    command = getExe pkgs.nixd;
    settings.nixd = {
      nixpkgs.expr = "import <nixpkgs> {}";
      formatting.command = [ (getExe pkgs.nixfmt) ];
    };
    initializationOptions.nixd = {
      nixpkgs.expr = "import <nixpkgs> {}";
      formatting.command = [ (getExe pkgs.nixfmt) ];
    };
    languages = toLang "nix" [ ".nix" ];
  };

  ruff = {
    command = getExe pkgs.ruff;
    args = [ "server" ];
    languages = toLang "python" [
      ".py"
      ".pyi"
      ".pyw"
    ];
  };

  # Resolved from PATH so a project's dev shell can supply the rust-analyzer
  # matching its toolchain; ./rust.nix provides the fallback.
  rust-analyzer = {
    command = "rust-analyzer";
    initializationOptions = {
      rust.analyzerTargetDir = true;
      check.command = "clippy";
    };
    languages = toLang "rust" [ ".rs" ];
  };

  taplo = {
    command = getExe pkgs.taplo;
    args = [
      "lsp"
      "stdio"
    ];
    languages = toLang "toml" [ ".toml" ];
  };

  texlab = {
    command = getExe pkgs.texlab;
    languages = {
      ".bib" = "bibtex";
      ".cls" = "latex";
      ".sty" = "latex";
      ".tex" = "latex";
    };
  };

  tinymist = {
    command = getExe pkgs.tinymist;
    settings = {
      exportPdf = "onSave";
      outputPath = "$root/$name";
      formatterMode = "typstyle";
    };
    languages = toLang "typst" [ ".typ" ];
  };

  ty = {
    command = getExe pkgs.ty;
    args = [ "server" ];
    languages = toLang "python" [
      ".py"
      ".pyi"
      ".pyw"
    ];
  };

  vscode-css-language-server = {
    command = getExe' pkgs.vscode-langservers-extracted "vscode-css-language-server";
    args = [ "--stdio" ];
    languages = {
      ".css" = "css";
      ".scss" = "scss";
      ".less" = "less";
    };
  };

  vscode-html-language-server = {
    command = getExe' pkgs.vscode-langservers-extracted "vscode-html-language-server";
    args = [ "--stdio" ];
    languages = toLang "html" [
      ".html"
      ".htm"
    ];
  };

  yaml-language-server = {
    command = getExe pkgs.yaml-language-server;
    args = [ "--stdio" ];
    languages = toLang "yaml" [
      ".yaml"
      ".yml"
    ];
  };

  # A server without `command` is left for zed to resolve on its own.
  renderZed =
    server:
    optionalAttrs (server ? command) {
      binary = {
        path = server.command;
      }
      // optionalAttrs (server ? args) { arguments = server.args; };
    }
    // optionalAttrs (server ? settings) { inherit (server) settings; }
    // optionalAttrs (server ? initializationOptions) {
      initialization_options = server.initializationOptions;
    };

  renderClaude =
    server:
    {
      inherit (server) command;
      extensionToLanguage = server.languages;
    }
    // optionalAttrs (server ? args) { inherit (server) args; }
    // optionalAttrs (server ? initializationOptions) { inherit (server) initializationOptions; }
    // optionalAttrs (server ? startupTimeout) { inherit (server) startupTimeout; };
in
{
  zed = mapAttrs (_: renderZed) {
    inherit
      bash-language-server
      clangd
      harper-ls
      jetls
      json-language-server
      lua-language-server
      nixd
      ruff
      taplo
      texlab
      ty
      vscode-css-language-server
      vscode-html-language-server
      yaml-language-server
      ;

    rust-analyzer = removeAttrs rust-analyzer [ "command" ];

    # Serves a preview on 127.0.0.1:23635.
    tinymist = recursiveUpdate tinymist {
      initializationOptions.preview.background.enabled = true;
    };
  };

  claude = mapAttrs (_: renderClaude) {
    inherit
      bash-language-server
      clangd
      cmake-language-server
      fish-lsp
      json-language-server
      julia-lsp
      lua-language-server
      marksman
      nixd
      ruff
      rust-analyzer
      taplo
      texlab
      tinymist
      ty
      vscode-css-language-server
      vscode-html-language-server
      yaml-language-server
      ;
  };
}
