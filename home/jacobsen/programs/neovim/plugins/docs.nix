{ pkgs }:
with pkgs.vimPlugins;
[
  {
    plugin = jupytext-nvim;
    type = "lua";
    config = /* lua */ ''
      -- Edits '.ipynb' as plain '# %%'-delimited text via the 'jupytext' CLI.
      -- Uses `now()` (not `later()`) so it also works when started like `nvim nb.ipynb`.
      now_if_args(function() require('jupytext').setup({ style = 'hydrogen' }) end)
    '';
  }
  {
    plugin = render-markdown-nvim;
    type = "lua";
    config = /* lua */ ''
      later(function() require('render-markdown').setup({}) end)
    '';
  }
  {
    plugin = typst-preview-nvim;
    type = "lua";
    config = /* lua */ ''
      later(function()
        require('typst-preview').setup({
          port = 0,
          follow_cursor = true,
          dependencies_bin = { ['tinymist'] = 'tinymist', ['websocat'] = 'websocat' },
        })
      end)
    '';
  }
]
