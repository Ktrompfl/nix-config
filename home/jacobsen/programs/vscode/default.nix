{
  config,
  lib,
  pkgs,
  ...
}:
let
  julia-apps = "${config.directory}/.julia/bin";
  jetls = "${julia-apps}/jetls";
  testrunner = "${julia-apps}/testrunner";

  general = {
    "extensions.autoCheckUpdates" = false;
    "extensions.autoUpdate" = false;
    "update.mode" = "none";
    "update.showReleaseNotes" = false;
    "redhat.telemetry.enabled" = false;
    "telemetry.telemetryLevel" = "off";
    "security.workspace.trust.enabled" = false;
    "chat.disableAIFeatures" = true;
  };

  window = {
    "window.dialogStyle" = "custom";
    "window.titleBarStyle" = "custom";
  };

  files = {
    "files.autoSave" = "onWindowChange";
    "files.insertFinalNewline" = true;
    "files.trimTrailingWhitespace" = true;
  };

  editor = {
    "editor.bracketPairColorization.enabled" = true;
    "editor.bracketPairColorization.independentColorPoolPerBracketType" = true;
    "editor.cursorBlinking" = "smooth";
    "editor.cursorSmoothCaretAnimation" = "on";
    "editor.codeActionsOnSave"."source.fixAll" = "always";
    "editor.fontLigatures" = true;
    "editor.fontWeight" = "500";
    "editor.formatOnPaste" = true;
    "editor.formatOnSave" = true;
    "editor.formatOnType" = true;
    "editor.guides.bracketPairs" = "active";
    "editor.guides.bracketPairsHorizontal" = "active";
    "editor.guides.indentation" = true;
    "editor.inlayHints.enabled" = "on";
    "editor.inlayHints.padding" = true;
    "editor.inlineSuggest.enabled" = true;
    "editor.linkedEditing" = true;
    "editor.lineNumbers" = "on";
    "editor.minimap.enabled" = false;
    "editor.parameterHints.enabled" = true;
    "editor.scrollbar.horizontal" = "hidden";
    "editor.semanticHighlighting.enabled" = true;
    "editor.showUnused" = true;
    "editor.snippetSuggestions" = "top";
    "editor.stickyScroll.enabled" = true;
    "editor.tabCompletion" = "on";
    "editor.tabSize" = 4;
    "editor.trimAutoWhitespace" = true;
    "editor.wordWrap" = "on";
    "editor.wrappingIndent" = "indent";
  };

  workbench = {
    "workbench.activityBar.location" = "top";
    "workbench.editor.empty.hint" = "hidden";
    "workbench.sideBar.location" = "left";
    "workbench.startupEditor" = "none";
    "workbench.tree.indent" = 16;
    "workbench.layoutControl.enabled" = false;
    "window.titleBarStyle" = "custom"; # without this vscode crashes on startup on wayland
  };

  terminal = {
    "terminal.integrated.gpuAcceleration" = "on";
    "terminal.integrated.minimumContrastRatio" = 1;
    "terminal.integrated.shell.linux" = (lib.getExe pkgs.fish);
  };

  # Extension settings
  extension = {
    git = {
      autofetch = true;
      # enableCommitSigning = true;
      enableSmartCommit = true;
      openRepositoryInParentFolders = "always";
    };

    errorLens = {
      gutterIconsEnabled = true;
      gutterIconSet = "defaultOutline";
    };

    eslint = {
      format.enable = true;
      problems.shortenToSingleLine = true;
      validate = [
        "javascript"
        "typescript"
        "javascriptreact"
        "typescriptreact"
        "astro"
      ];
    };

    prettier.jsxSingleQuote = true;
  };

  # Formatter settings
  formatter = {
    "[astro]"."editor.defaultFormatter" = "esbenp.prettier-vscode";
    "[css]"."editor.defaultFormatter" = "esbenp.prettier-vscode";
    "[html]"."editor.defaultFormatter" = "esbenp.prettier-vscode";
    "[javascript]"."editor.defaultFormatter" = "esbenp.prettier-vscode";
    "[json]"."editor.defaultFormatter" = "esbenp.prettier-vscode";
    "[jsonc]"."editor.defaultFormatter" = "esbenp.prettier-vscode";
    "[markdown]"."editor.defaultFormatter" = "esbenp.prettier-vscode";
    "[nix]"."editor.defaultFormatter" = "jnoortheen.nix-ide";
    "[scss]"."editor.defaultFormatter" = "esbenp.prettier-vscode";
    "[typescript]"."editor.defaultFormatter" = "esbenp.prettier-vscode";
    "[typescriptreact]"."editor.defaultFormatter" = "esbenp.prettier-vscode";
  };

  # Language specific settings
  language = import ./languages.nix {
    inherit
      jetls
      lib
      pkgs
      testrunner
      ;
  };

  declarativeSettings =
    general // window // files // editor // workbench // terminal // extension // formatter // language;

  extensions = import ./extensions.nix { inherit pkgs; };
in
{
  packages = [
    (pkgs.vscode-with-extensions.override { vscodeExtensions = extensions; })
  ];

  xdg.config.files."Code/User/settings.json" = {
    type = "copy";
    clobber = true;
    generator = (pkgs.formats.json { }).generate "vscode-settings.json";
    value = declarativeSettings;
  };

  # electron apps store their state inseparably in .config/
  preservation.preserveAt.state-dir.directories = [ ".config/Code" ];
}
