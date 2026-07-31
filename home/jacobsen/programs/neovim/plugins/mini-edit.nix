{ pkgs }:
with pkgs.vimPlugins;
[
  {
    plugin = mini-ai;
    type = "lua";
    config = /* lua */ ''
      -- a/i textobjects, e.g. `ci)`, `yaq`, `vif`, `cina` (next argument)
      later(function()
        local ai = require('mini.ai')
        ai.setup({
          custom_textobjects = {
            B = MiniExtra.gen_ai_spec.buffer(),
            F = ai.gen_spec.treesitter({ a = '@function.outer', i = '@function.inner' }),
          },
          search_method = 'cover', -- only match the textobject covering the cursor
        })
      end)
    '';
  }
  {
    plugin = mini-align;
    type = "lua";
    config = /* lua */ ''
      -- `ga`/`gA` (interactive) align operators
      later(function() require('mini.align').setup() end)
    '';
  }
  {
    plugin = mini-bracketed;
    type = "lua";
    config = /* lua */ ''
      -- `[`/`]` + target (b buffer, c conflict, d diagnostic, ...) navigation
      later(function() require('mini.bracketed').setup() end)
    '';
  }
  {
    plugin = mini-bufremove;
    type = "lua";
    config = /* lua */ ''
      later(function() require('mini.bufremove').setup() end)
    '';
  }
  {
    plugin = mini-comment;
    type = "lua";
    config = /* lua */ ''
      -- `gc` comment operator, e.g. `gcip`
      later(function() require('mini.comment').setup() end)
    '';
  }
  {
    plugin = mini-completion;
    type = "lua";
    config = /* lua */ ''
      later(function()
        -- Don't show noisy 'Text' suggestions; show snippets last
        local process_items_opts = { kind_priority = { Text = -1, Snippet = 99 } }
        local process_items = function(items, base)
          return MiniCompletion.default_process_items(items, base, process_items_opts)
        end
        require('mini.completion').setup({
          lsp_completion = { source_func = 'omnifunc', auto_setup = false, process_items = process_items },
        })
        new_autocmd('LspAttach', nil, function(ev)
          vim.bo[ev.buf].omnifunc = 'v:lua.MiniCompletion.completefunc_lsp'
        end, "Set 'omnifunc'")
        vim.lsp.config('*', { capabilities = MiniCompletion.get_lsp_capabilities() })
      end)
    '';
  }
  {
    plugin = mini-jump2d;
    type = "lua";
    config = /* lua */ ''
      -- `<CR>` labels every visible jump spot; type the label to land on it. Also works as
      -- an operator target, e.g. `d<CR>` deletes up to the chosen spot.
      later(function() require('mini.jump2d').setup() end)
    '';
  }
  {
    plugin = mini-keymap;
    type = "lua";
    config = /* lua */ ''
      -- Navigate 'mini.completion' menu with `<Tab>`/`<S-Tab>`; account for 'mini.pairs' on `<CR>`/`<BS>`
      later(function()
        require('mini.keymap').setup()
        MiniKeymap.map_multistep('i', '<Tab>', { 'pmenu_next' })
        MiniKeymap.map_multistep('i', '<S-Tab>', { 'pmenu_prev' })
        MiniKeymap.map_multistep('i', '<CR>', { 'pmenu_accept', 'minipairs_cr' })
        MiniKeymap.map_multistep('i', '<BS>', { 'minipairs_bs' })
      end)
    '';
  }
  {
    plugin = mini-move;
    type = "lua";
    config = /* lua */ ''
      -- `<M-hjkl>` moves the current line/selection
      later(function() require('mini.move').setup() end)
    '';
  }
  {
    plugin = mini-operators;
    type = "lua";
    config = /* lua */ ''
      -- `gr` replace, `gx` exchange, `gm` multiply, `gs` sort, `g=` evaluate-as-Lua, e.g.
      -- `griw` replaces inside word. Auto-remaps built-in `gx` (open URL) to `gX`.
      later(function() require('mini.operators').setup() end)
    '';
  }
  {
    plugin = mini-pairs;
    type = "lua";
    config = /* lua */ ''
      later(function() require('mini.pairs').setup({ modes = { command = true } }) end)
    '';
  }
  {
    plugin = mini-splitjoin;
    type = "lua";
    config = /* lua */ ''
      -- `gS` toggles single-line/multi-line arguments
      later(function() require('mini.splitjoin').setup() end)
    '';
  }
  {
    plugin = mini-surround;
    type = "lua";
    config = /* lua */ ''
      -- `sa`/`sd`/`sr`/`sf`/`sh` add/delete/replace/find/highlight surroundings, e.g. `saiw)`
      later(function() require('mini.surround').setup() end)
    '';
  }
  {
    plugin = mini-trailspace;
    type = "lua";
    config = /* lua */ ''
      later(function() require('mini.trailspace').setup() end)
    '';
  }
]
