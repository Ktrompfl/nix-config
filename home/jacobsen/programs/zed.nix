{
  config,
  inputs,
  lib,
  pkgs,
  ...
}:
{
  programs.zed-editor = {
    enable = true;
    extensions = [
      "csv"
      "fish"
      "haskell"
      "html"
      "ini"
      # if the julia lsp fails, update the zed environment via 'julia --project=@zed-julia'
      "julia"
      "latex"
      "log"
      "lua"
      "make"
      "nix"
      "python"
      "toml"
      "typst"
      "rust"
      "xml"
    ];

    userSettings = {
      agent = {
        dock = "right";
        show_turn_stats = true;
        sidebar_side = "right";
      };
      agent_servers = {
        claude-acp = {
          # type = "registry"; # latest, standalone acp adapter
          type = "custom";
          command = lib.getExe inputs.llm-agents.packages.${pkgs.stdenv.hostPlatform.system}.claude-agent-acp;
          env = {
            # use wrapped claude code package to make configured plugins (e.g. language servers) available
            CLAUDE_CODE_EXECUTABLE = lib.getExe config.programs.claude-code.finalPackage;
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
      focus_follows_mouse.enabled = true;
      git_panel.dock = "left";
      indent_guides = {
        active_line_width = 1;
        background_coloring = "disabled";
        coloring = "fixed";
        line_width = 1;
      };
      inlay_hints.enabled = true;
      journal.hour_format = "hour24";
      languages = {
        Nix = {
          formatter = {
            external = {
              command = lib.getExe pkgs.nixfmt;
            };
          };
          language_servers = [
            "!nil"
            "nixd"
          ];
          tab_size = 2;
        };
      };
      load_direnv = "shell_hook";
      lsp = {
        basedpyright = {
          binary.path = lib.getExe pkgs.basedpyright;
        };
        clangd = {
          binary.path = lib.getExe' pkgs.clang-tools "clangd";
        };
        nixd = {
          binary.path = lib.getExe pkgs.nixd;
          nixpkgs.expr = "import <nixpkgs> {}";
          formatting.command = [ (lib.getExe pkgs.nixfmt) ];
        };
        tinymist = {
          binary.path = lib.getExe pkgs.tinymist;
        };
        ruff = {
          binary.path = lib.getExe pkgs.ruff;
        };
        rust-analyzer = {
          binary.path = lib.getExe pkgs.rust-analyzer;
        };
      };
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
  };

  preservation.preserveAt.state-dir.directories = [ ".local/share/zed" ];
}
