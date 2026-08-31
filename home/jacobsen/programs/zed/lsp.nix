{
  lib,
  pkgs,
  ...
}:
{
  clangd = {
    binary.path = lib.getExe' pkgs.clang-tools "clangd";
  };
  harper-ls = {
    binary = {
      path = lib.getExe' pkgs.harper "harper-ls";
      arguments = [ "--stdio" ];
    };
    settings.harper-ls = {
      dialect = "British";
    };
  };
  jetls = {
    settings = {
      code_lens.references = true;
      formatter.custom = {
        executable = lib.getExe pkgs.runic;
        executable_range = lib.getExe pkgs.runic;
      };
    };
  };
  lua-language-server = {
    binary.path = lib.getExe pkgs.lua-language-server;
  };
  nixd = {
    binary.path = lib.getExe pkgs.nixd;
    settings.nixd = {
      nixpkgs.expr = "import <nixpkgs> {}";
      formatting.command = [ (lib.getExe pkgs.nixfmt) ];
    };
  };
  texlab = {
    binary.path = lib.getExe pkgs.texlab;
  };
  tinymist = {
    binary.path = lib.getExe pkgs.tinymist;
    initialization_options = {
      # run preview server on 127.0.0.1:23635
      preview.background = {
        enabled = true;
      };
    };
    settings = {
      exportPdf = "onSave";
      outputPath = "$root/$name";
      formatterMode = "typstyle";
    };
  };
  ty = {
    binary = {
      path = lib.getExe pkgs.ty;
      arguments = [ "server" ];
    };
  };
  ruff = {
    binary = {
      path = lib.getExe pkgs.ruff;
      arguments = [ "server" ];
    };
  };
  rust-analyzer = {
    initialization_options = {
      rust.analyzerTargetDir = true;
      check.command = "clippy";
    };
  };
}
