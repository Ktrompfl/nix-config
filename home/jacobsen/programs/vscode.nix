{ lib, pkgs, ... }:
{
  home.packages = [ pkgs.haskell-language-server ];

  programs.vscode = {
    enable = true;
    profiles.default = {
      enableUpdateCheck = false;
      enableExtensionUpdateCheck = false;
    };
    haskell = {
      enable = true;
      hie.enable = false;
    };

    mutableExtensionsDir = false;
    profiles.default.extensions = with pkgs.vscode-extensions; [
      # if any extension is ever outdated again switch to  https://github.com/nix-community/nix-vscode-extensions
      dbaeumer.vscode-eslint
      eamodio.gitlens
      esbenp.prettier-vscode
      formulahendry.code-runner
      golang.go
      gruntfuggly.todo-tree
      haskell.haskell
      james-yu.latex-workshop
      jnoortheen.nix-ide
      justusadam.language-haskell # required for haskell
      julialang.language-julia
      redhat.java
      redhat.vscode-xml
      redhat.vscode-yaml
      mechatroner.rainbow-csv
      mhutchie.git-graph
      ms-azuretools.vscode-docker
      ms-python.black-formatter # python formatter
      ms-python.python
      ms-toolsai.jupyter
      ms-vscode.cpptools
      myriad-dreamin.tinymist # combined typst lsp and preview
      naumovs.color-highlight
      streetsidesoftware.code-spell-checker
      sumneko.lua
      tamasfe.even-better-toml
      usernamehw.errorlens
      valentjn.vscode-ltex
    ];
  };

  programs.vscode.profiles.default.userSettings =
    let
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
      language = {
        go = {
          inlayHints = {
            assignVariableTypes = true;
            compositeLiteralFields = true;
            compositeLiteralTypes = true;
            constantValues = true;
            functionTypeParameters = true;
            parameterNames = true;
            rangeVariableTypes = true;
          };
          lintTool = "golangci-lint";
          useLanguageServer = true;
        };

        # haskell.serverExecutablePath = "${pkgs.haskell-language-server}/bin/haskell-language-server-wrapper";  # just added to path :/

        javascript = {
          inlayHints = {
            functionLikeReturnTypes.enabled = true;
            parameterNames.enabled = "all";
            parameterTypes.enabled = true;
            propertyDeclarationTypes.enabled = true;
          };
          preferGoToSourceDefinition = true;
          completeFunctionCalls = true;
        };

        julia = {
          symbolCacheDownload = false; # this just keeps on downloading and spinning up new processes, filling up memory
          executablePath = "${lib.getExe pkgs.julia-lts}";
          enableTelemetry = false;
          NumThreads = "auto";
        };

        latex-workshop.latex = {
          outDir = "%DIR%/out";
          recipe.default = "lastUsed";
        };

        ltex = {
          additionalRules.motherTongue = "de-DE";
          ltex.language = "de-DE";
        };

        nix = {
          enableLanguageServer = true;
          formatterPath = "${lib.getExe pkgs.nixfmt}";
          serverPath = "${lib.getExe pkgs.nil}";
          serverSettings = {
            nil.formatting.command = [ "${lib.getExe pkgs.nixfmt}" ];
            nixd.formatting.command = [ "${lib.getExe pkgs.nixfmt}" ];
          };
        };

        python.terminal.executeInFileDir = true;

        typescript = {
          inlayHints = {
            functionLikeReturnTypes.enabled = true;
            parameterNames.enabled = "all";
            parameterTypes.enabled = true;
            propertyDeclarationTypes.enabled = true;
          };
          preferGoToSourceDefinition = true;
          suggest.completeFunctionCalls = true;
        };

        # typst (tinymist)
        tinymist = {
          tinymist.preview.cursorIndicator = true;
          exportPdf = "onDocumentHasTitle";
          formatterMode = "typstyle";
        };
      };
    in
    general // window // files // editor // workbench // terminal // extension // formatter // language;

  # electron apps store their state inseparably in .config/
  preservation.preserveAt.state-dir.directories = [
    ".config/Code"
  ];

  xdg.mimeApps.defaultApplications = {
    "text/plain" = "code.desktop";
    "application/x-zerosize" = "code.desktop";
  };
}
