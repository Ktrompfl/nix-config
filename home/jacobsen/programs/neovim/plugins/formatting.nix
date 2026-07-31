{ pkgs }:
with pkgs.vimPlugins;
[
  {
    plugin = conform-nvim;
    type = "lua";
    config = /* lua */ ''
      later(function()
        require('conform').setup({
          format_on_save = { lsp_fallback = true, timeout_ms = 2000 },
          formatters_by_ft = {
            julia = { 'runic' },
            lua = { 'stylua' },
            nix = { 'nixfmt' },
            plaintex = { 'latexindent' },
            python = { 'ruff_organize_imports', 'ruff_format' },
            rust = { 'rustfmt' },
            tex = { 'latexindent' },
          },
        })
      end)
    '';
  }
]
