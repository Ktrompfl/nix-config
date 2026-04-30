{
  programs.zed = {
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

    userSettings = {
      auto_update = false;

      autosave = "on_window_change";
      auto_signature_help = true; # Show method signatures in the editor, when inside parentheses.
      base_keymap = "VSCode";
      ensure_final_newline_on_save = true;
      inlay_hints.enable = true;
      journal.hour_format = "hour24";
      load_direnv = "shell_hook"; # load direnv configuration through the shell hook, works for POSIX shells and fish
      vim_mode = false;

      # disable telemetry
      telemetry = {
        diagnostics = false;
        metrics = false;
      };
    };
  };
}
