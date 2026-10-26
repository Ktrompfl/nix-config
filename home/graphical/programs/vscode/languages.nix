{ lib, pkgs }:
{
  # julia: the client installs and updates jetls itself
  jetls-client.settings.formatter.custom = {
    executable = lib.getExe pkgs.runic;
    executable_range = lib.getExe pkgs.runic;
  };

  # jupyter
  jupyter.askForKernelRestart = false;
  notebook = {
    formatOnSave.enabled = true;
    output = {
      scrolling = true;
      wordWrap = true;
    };
    stickyScroll.enabled = true;
  };

  # nix
  nix = {
    enableLanguageServer = true;
    formatterPath = lib.getExe pkgs.nixfmt;
    serverPath = lib.getExe pkgs.nixd;
    serverSettings.nixd = {
      nixpkgs.expr = "import <nixpkgs> {}";
      formatting.command = [ (lib.getExe pkgs.nixfmt) ];
    };
  };
  "[nix]" = {
    "editor.defaultFormatter" = "jnoortheen.nix-ide";
    "editor.tabSize" = 2;
  };

  # python: ruff formats and lints, ty does the rest
  python = {
    languageServer = "None";
    terminal.executeInFileDir = true;
  };
  "[python]" = {
    "editor.defaultFormatter" = "charliermarsh.ruff";
    # not fixAll, which deletes imports that are not used yet
    "editor.codeActionsOnSave"."source.organizeImports" = "explicit";
  };

  # typst
  tinymist = {
    exportPdf = "onSave";
    formatterMode = "typstyle";
    outputPath = "$root/$name";
    preview.cursorIndicator = true;
  };
}
