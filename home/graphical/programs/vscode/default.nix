{
  config,
  lib,
  pkgs,
  ...
}:
let
  # A minimal editor for teaching beginners; zed is the one for daily work.

  general = {
    "chat.commandCenter.enabled" = false;
    "chat.disableAIFeatures" = true;
    "extensions.autoCheckUpdates" = false;
    "extensions.autoUpdate" = false;
    "extensions.ignoreRecommendations" = true;
    "security.workspace.trust.enabled" = false;
    "telemetry.telemetryLevel" = "off";
    "update.mode" = "none";
    "update.showReleaseNotes" = false;
    "workbench.enableExperiments" = false;
  };

  window = {
    "window.commandCenter" = false;
    "window.dialogStyle" = "custom";
    "window.menuBarVisibility" = "compact";
    "window.titleBarStyle" = "custom"; # without this vscode crashes on startup on wayland
  };

  files = {
    "files.autoSave" = "onFocusChange";
    "files.exclude" = lib.genAttrs [
      # the defaults, which this replaces
      "**/.git"
      "**/.svn"
      "**/.hg"
      "**/.DS_Store"
      "**/Thumbs.db"

      "**/.direnv"
      "**/.ipynb_checkpoints"
      "**/.pytest_cache"
      "**/.ruff_cache"
      "**/__pycache__"
    ] (_: true);
    "files.insertFinalNewline" = true;
    "files.trimFinalNewlines" = true;
    "files.trimTrailingWhitespace" = true;
  };

  editor = {
    "editor.bracketPairColorization.independentColorPoolPerBracketType" = true;
    "editor.codeLens" = false;
    "editor.cursorBlinking" = "smooth";
    "editor.cursorSmoothCaretAnimation" = "on";
    "editor.fontFamily" = "'${config.theme.fonts.monospace.name}'";
    # beginners should see the characters they type, not `≠` for `!=`
    "editor.fontLigatures" = false;
    "editor.fontSize" = 16;
    "editor.fontWeight" = "500";
    "editor.formatOnPaste" = true;
    "editor.formatOnSave" = true;
    "editor.guides.bracketPairs" = "active";
    # hold ctrl+alt to show them
    "editor.inlayHints.enabled" = "offUnlessPressed";
    "editor.linkedEditing" = true;
    "editor.minimap.enabled" = false;
    "editor.mouseWheelZoom" = true;
    "editor.renderWhitespace" = "selection";
    "editor.scrollbar.horizontal" = "hidden";
    "editor.stickyScroll.enabled" = true;
    "editor.wordWrap" = "on";
    "editor.wrappingIndent" = "indent";
  };

  # As in zed: everything docked left, nothing in the title bar.
  workbench = {
    "workbench.activityBar.location" = "top";
    "workbench.colorTheme" = pkgs.tinted-vscode.themeName config.theme.colors;
    # single clicks open files for good instead of in a replaceable preview tab
    "workbench.editor.enablePreview" = false;
    "workbench.editor.empty.hint" = "hidden";
    "workbench.layoutControl.enabled" = false;
    "workbench.navigationControl.enabled" = false;
    "workbench.secondarySideBar.defaultVisibility" = "hidden";
    "workbench.sideBar.location" = "left";
    "workbench.startupEditor" = "none";
    "workbench.tips.enabled" = false;
    "workbench.tree.indent" = 16;
    "workbench.welcomePage.walkthroughs.openOnInstall" = false;
  };

  terminal = {
    "terminal.integrated.copyOnSelection" = true;
    "terminal.integrated.defaultProfile.linux" = "fish";
    "terminal.integrated.fontSize" = 16;
    "terminal.integrated.gpuAcceleration" = "on";
    "terminal.integrated.minimumContrastRatio" = 1;
    "terminal.integrated.profiles.linux".fish.path = lib.getExe pkgs.fish;
  };

  # Extension settings
  extension = {
    direnv.restart.automatic = true;

    # spelling and grammar stay squiggles; errorlens repeats the rest inline
    errorLens.enabledDiagnosticLevels = [
      "error"
      "warning"
    ];
    harper.dialect = "British";

    git = {
      autofetch = true;
      confirmSync = false;
      enableSmartCommit = true;
      openRepositoryInParentFolders = "always";
    };
  };

  # Language specific settings
  language = import ./languages.nix { inherit lib pkgs; };

  declarativeSettings =
    general // window // files // editor // workbench // terminal // extension // language;

  extensions = import ./extensions.nix {
    inherit pkgs;
    palette = config.theme.colors;
  };
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
