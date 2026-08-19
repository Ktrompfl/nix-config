{
  jetls,
  lib,
  pkgs,
  testrunner,
}:
{
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

  "jetls-client" = {
    executable = {
      path = jetls;
      threads = "auto";
    };
    settings = {
      code_lens.references = true;
      formatter.custom = {
        executable = lib.getExe pkgs.runic;
        executable_range = lib.getExe pkgs.runic;
      };
      testrunner.executable = testrunner;
    };
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
}
