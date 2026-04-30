{ lib, pkgs, ... }:
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
      "ltex"
      "lua"
      "make"
      "nix"
      "python"
      "toml"
      "typst"
      "rust"
      "xml"
    ];

    extraPackages = with pkgs; [
      claude-code
    ];

    userSettings = {
      auto_update = false;
      auto_install_extensions = false;

      # disable telemetry
      telemetry = {
        diagnostics = false;
        metrics = false;
      };

      autosave = "on_focus_change";

      auto_signature_help = true;
      inlay_hints.enable = true;

      load_direnv = "shell_hook"; # load direnv configuration through the shell hook, works for POSIX shells and fish

      terminal = {
        shell.program = lib.getExe pkgs.fish;
      };

      journal.hour_format = "hour24";

      base_keymap = "VSCode";
      vim_mode = true;
      which_key.enable = true;

      # control ui elements
      agent.dock = "right";
      outline_panel.dock = "left";
      git_panel.dock = "left";
      collaboration_panel.button = false;
      collaboration_panel.dock = "right";
      title_bar = {
        show_sign_in = false;
        show_user_menu = false;
        show_user_picture = false;
      };

      languages = {
        Nix = {
          tab_size = 2;
          formatter.external.command = lib.getExe pkgs.nixfmt;
          language_servers = [
            "nixd"
            "!nil"
          ];
        };
      };

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
    };
  };
}
