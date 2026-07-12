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

        tool_permissions = {
          default = "confirm";
          tools = {
            search_web.default = "allow";
            fetch = {
              default = "confirm";
              always_allow = [
                { pattern = "^https://(github\\.com|raw\\.githubusercontent\\.com|devenv\\.sh)/"; }
                { "pattern" = "docs\\.rs"; }
              ];
            };

            create_directory.default = "allow";
            copy_path.default = "confirm";
            move_path.default = "confirm";
            delete_path = {
              default = "confirm";
              always_deny = [
                { pattern = "^/$"; }
              ];
            };

            edit_file = {
              default = "confirm";
              # protect sensitive files
              always_deny = [
                { pattern = "\\.env"; }
                { pattern = "secrets?/"; }
                { pattern = "\\.(pem|key)$"; }
              ];
            };
            write_file.default = "confirm";

            terminal = {
              default = "confirm";
              always_allow = [
                # Read-only git commands
                {
                  pattern = "^git (status|log|diff|show|branch|remote|blame|ls-files|rev-parse|describe|shortlog|reflog|cat-file|grep|ls-tree|show-ref|for-each-ref|rev-list|merge-base|name-rev)\\b";
                }
                {
                  pattern = "^git (stash list|worktree list|submodule status|config (--get|--list|-l))\\b";
                }

                # Safe file system operations
                {
                  pattern = "^(ls|find|fd|cat|head|tail|pwd|stat|file|wc|tree|realpath|readlink|dirname|basename|du|df)\\b";
                }

                # Safe read-only text/data inspection
                {
                  pattern = "^(rg|grep|diff|sort|uniq|cut|comm|column|jq|nl|tac|rev|tr)\\b";
                }
                { pattern = "^sed -n\\b"; }

                # Safe read-only binary/hash inspection
                {
                  pattern = "^(od|xxd|hexdump|strings|base64|cksum|md5sum|sha1sum|sha256sum|sha512sum|b2sum)\\b";
                }

                # Safe read-only system info
                {
                  pattern = "^(whoami|id|hostname|uname|date|uptime|env|printenv|which|type|getconf|free|ps|pgrep|lsof|ss|lscpu|lsblk|lsusb|lspci|findmnt|getent|groups|locale)\\b";
                }
                { pattern = "^command -v\\b"; }

                # Safe nix read operations
                {
                  pattern = "^nix (eval|flake (show|metadata|check)|search|log|path-info|derivation show|why-depends|store (ls|cat|info)|config show|show-config|registry list|profile list)\\b";
                }
                { pattern = "^nix-instantiate --parse\\b"; }
                { pattern = "^nix-store (-q|--query)\\b"; }
                { pattern = "^(nixos-option|statix check|nh search)\\b"; }

                # Cargo operations
                { pattern = "^cargo\\s+(check|build|test|clippy|fmt|doc)\\b"; }

                # Jujutsu read-only
                { pattern = "^jj (status|log|diff|show|evolog|op log|file list|bookmark list)\\b"; }

                # GitHub CLI read-only; gh api stays on confirm since it can mutate.
                {
                  pattern = "^gh (pr (view|list|diff|checks|status)|issue (view|list|status)|run (list|view)|repo view|release (list|view)|label list|search)\\b";
                }

                # Git staging, directory creation, misc read-only system info
                { pattern = "^git add\\b"; }
                { pattern = "^(mkdir|chmod)\\b"; }
                { pattern = "^systemctl (list-units|list-timers|status)\\b"; }
                { pattern = "^(journalctl|dmesg)\\b"; }
                { pattern = "^claude --version$"; }
                { pattern = "^coredumpctl list\\b"; }
              ];
              always_confirm = [
                # Potentially destructive git commands
                { pattern = "^git (checkout|commit|merge|pull|push|rebase|reset|restore|switch)\\b"; }
                { pattern = "^git stash($|\\s+(push|pop|apply|drop|clear|store|branch))"; }

                # File deletion and modification
                { pattern = "^(cp|mv|rm|dd|mkfs|shutdown|reboot)\\b"; }

                # System control operations
                { pattern = "^systemctl (disable|enable|mask|reload|restart|start|stop|unmask)\\b"; }

                # Network operations
                { pattern = "^(curl|ping|rsync|scp|ssh|wget)\\b"; }

                # Package management
                { pattern = "^nix (build|develop|run|shell)\\b"; }
                { pattern = "^(nixos-rebuild|sudo)\\b"; }

                # Process management
                { pattern = "^(kill|killall|pkill)\\b"; }
              ];
              always_deny = [
                { pattern = "^rm -rf /(\\*)?$"; }
              ];
            };
          };
        };
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
        julia =
          let
            julia = lib.getExe pkgs.julia-bin;
            julia-apps = "${config.home.homeDirectory}/.julia/bin";
            # Note: The julia executables must be installed manually with:
            # - julia -e 'using Pkg; Pkg.Apps.add(; url="https://github.com/aviatesk/JETLS.jl", rev="release")'
            # - julia -e 'using Pkg; Pkg.Apps.add("JuliaFormatter")'
            # - julia -e 'using Pkg; Pkg.Apps.add(; url="https://github.com/aviatesk/TestRunner.jl", rev="release")'
            jetls = "${julia-apps}/jetls";
            jlfmt = "${julia-apps}/jlfmt";
            testrunner = "${julia-apps}/testrunner";
          in
          {
            binary = {
              path = jetls;
              # arguments must be declared or zed shortcuts to []
              arguments = [
                "--threads=auto"
                "--"
                "serve"
              ];
              env.JULIA_APPS_JULIA_CMD = julia;
            };
            settings = {
              code_lens.references = true;
              formatter.custom = {
                executable = jlfmt;
                executable_range = jlfmt;
              };
              testrunner.executable = testrunner;
            };
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

  # manually installed extensions
  home.file.".local/share/zed/extensions/installed/julia".source = pkgs.zed-julia;

  preservation.preserveAt.state-dir.directories = [ ".local/share/zed" ];
}
