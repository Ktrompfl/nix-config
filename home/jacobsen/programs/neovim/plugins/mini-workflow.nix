{ pkgs }:
with pkgs.vimPlugins;
[
  {
    plugin = mini-diff;
    type = "lua";
    config = /* lua */ ''
      -- In-buffer diff hunks vs Git index; `gh`/`gH` apply/reset, `<Leader>go` toggles overlay
      later(function() require('mini.diff').setup() end)
    '';
  }
  {
    plugin = mini-files;
    type = "lua";
    config = /* lua */ ''
      -- Miller-columns file browser, edited as text; `<Leader>ed`/`<Leader>ef` open it
      later(function()
        require('mini.files').setup({ windows = { preview = true } })
        new_autocmd('User', 'MiniFilesExplorerOpen', function()
          MiniFiles.set_bookmark('c', vim.fn.stdpath('config'), { desc = 'Config' })
          MiniFiles.set_bookmark('w', vim.fn.getcwd, { desc = 'Working directory' })
        end, 'Add bookmarks')
      end)
    '';
  }
  {
    plugin = mini-git;
    type = "lua";
    config = /* lua */ ''
      -- `:Git` command + `<Leader>gs` show-at-cursor; not a full git client, mini.diff covers hunks
      later(function() require('mini.git').setup() end)
    '';
  }
  {
    plugin = mini-pick;
    type = "lua";
    config = /* lua */ ''
      -- Fuzzy picker (fzf-equivalent) backed by ripgrep; see the `<Leader>f*` mappings
      later(function() require('mini.pick').setup() end)
    '';
  }
  {
    plugin = mini-visits;
    type = "lua";
    config = /* lua */ ''
      -- Frecency-tracked file visits, `<Leader>fv`/`<Leader>v*`
      later(function() require('mini.visits').setup() end)
    '';
  }
]
