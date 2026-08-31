{
  config,
  inputs,
  lib,
  osConfig,
  pkgs,
  ...
}:
let
  claudeCode = inputs.llm-agents.packages.${pkgs.stdenv.hostPlatform.system}.claude-code;

  # Zed reads MCP servers from its own settings; the secret is passed through a
  # wrapper because zed has no `{file:...}` expansion of its own.
  context7 = pkgs.writeShellScriptBin "mcp-context7-wrapper" ''
    export CONTEXT7_API_KEY="$(cat ${osConfig.sops.secrets."api-keys/context7".path})"
    exec ${lib.getExe pkgs.context7-mcp} "$@"
  '';
  settings = {
    context_servers = {
      context7 = {
        command = lib.getExe context7;
        args = [ ];
        enabled = true;
      };
      nixos = {
        command = lib.getExe pkgs.mcp-nixos;
        args = [ ];
        enabled = true;
      };
    };

    buffer_font_family = config.theme.fonts.monospace.name;
    buffer_font_size = 16;
    ui_font_family = config.theme.fonts.sansSerif.name;
    ui_font_size = 16;
    theme = "Base16 Rosé Pine";

    auto_install_extensions = {
      "csv" = true;
      "harper" = true;
      "html" = true;
      "ini" = true;
      "julia" = true;
      "latex" = true;
      "log" = true;
      "lua" = true;
      "make" = true;
      "nix" = true;
      "python" = true;
      "rust" = true;
      "toml" = true;
      "typst" = true;
      "xml" = true;
    };

    agent = import ./agent.nix;
    agent_servers = {
      claude-acp = {
        # type = "registry"; # latest, standalone acp adapter
        type = "custom";
        command = lib.getExe inputs.llm-agents.packages.${pkgs.stdenv.hostPlatform.system}.claude-agent-acp;
        env = {
          # use wrapped claude code package to make configured plugins (e.g. language servers) available
          CLAUDE_CODE_EXECUTABLE = lib.getExe claudeCode;
        };
      };
    };
    auto_signature_help = true;
    auto_update = false;
    autosave = "on_focus_change";
    base_keymap = "VSCode";
    cli_default_open_behavior = "new_window";
    code_lens = "on";
    collaboration_panel = {
      button = false;
      dock = "right";
    };
    colorize_brackets = true;
    completion_menu_item_kind = "symbol";
    debugger.button = false;
    diagnostics.inline.enabled = true;
    document_folding_ranges = "off";
    format_on_save = "on";
    git_panel.dock = "left";
    indent_guides = {
      active_line_width = 1;
      background_coloring = "disabled";
      coloring = "fixed";
      line_width = 1;
    };
    inlay_hints.enabled = true;
    journal.hour_format = "hour24";

    jupyter = {
      enabled = true;
      # fallback kernels (overwritten by project local environments)
      kernel_selections = {
        julia = "julia-nixpkgs";
        python = "python3-nixpkgs";
      };
    };

    languages = import ./languages.nix { inherit lib pkgs; };
    load_direnv = "shell_hook";
    lsp = import ./lsp.nix { inherit config lib pkgs; };
    node = {
      path = lib.getExe pkgs.nodejs;
      npm_path = lib.getExe' pkgs.nodejs "npm";
    };
    outline_panel.dock = "left";
    project_panel = {
      dock = "left";
      git_status_indicator = false;
      hide_gitignore = false;
    };
    repl = {
      max_columns = 128;
      max_lines = 64;
    };
    semantic_tokens = "combined";
    show_signature_help_after_edits = true;
    show_whitespaces = "selection";
    soft_wrap = "bounded";
    status_bar = {
      line_endings_button = false;
      show_active_file = true;
    };
    sticky_scroll.enabled = true;
    tabs = {
      file_icons = true;
      git_status = true;
    };
    telemetry = {
      diagnostics = false;
      metrics = false;
    };
    terminal = {
      copy_on_select = true;
      dock = "bottom";
      shell.program = lib.getExe pkgs.fish;
      show_count_badge = false;
    };
    title_bar = {
      show_branch_status_icon = false;
      show_menus = false;
      show_sign_in = false;
      show_user_menu = false;
      show_user_picture = false;
    };
    toolbar.code_actions = true;
    unnecessary_code_fade = 0.4;
    vim.use_smartcase_find = true;
    vim_mode = true;
    which_key.enabled = true;
  };
in
{
  packages = [
    pkgs.zed-editor
    pkgs.texlive.bin.latexindent
  ];

  xdg.config.files."zed/themes/tinted.json".source = pkgs.tinted-zed.themeFor config.theme.colors;

  xdg.config.files."zed/settings.json" = {
    type = "copy";
    clobber = true;
    generator = (pkgs.formats.json { }).generate "zed-settings.json";
    value = settings;
  };

  preservation.preserveAt.state-dir.directories = [ ".local/share/zed" ];
}
