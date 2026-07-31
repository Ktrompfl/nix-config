{ pkgs }:
with pkgs.vimPlugins;
[
  {
    plugin = mini-statusline;
    type = "lua";
    config = /* lua */ ''
      now(function() require('mini.statusline').setup() end)
    '';
  }
  {
    plugin = mini-tabline;
    type = "lua";
    config = /* lua */ ''
      now(function() require('mini.tabline').setup() end)
    '';
  }
  {
    plugin = mini-cursorword;
    type = "lua";
    config = /* lua */ ''
      later(function() require('mini.cursorword').setup() end)
    '';
  }
  {
    plugin = mini-indentscope;
    type = "lua";
    config = /* lua */ ''
      later(function() require('mini.indentscope').setup() end)
    '';
  }
  {
    plugin = mini-hipatterns;
    type = "lua";
    config = /* lua */ ''
      later(function()
        local hipatterns = require('mini.hipatterns')
        local hi_words = MiniExtra.gen_highlighter.words
        hipatterns.setup({
          highlighters = {
            fixme = hi_words({ 'FIXME', 'Fixme', 'fixme' }, 'MiniHipatternsFixme'),
            hack = hi_words({ 'HACK', 'Hack', 'hack' }, 'MiniHipatternsHack'),
            todo = hi_words({ 'TODO', 'Todo', 'todo' }, 'MiniHipatternsTodo'),
            note = hi_words({ 'NOTE', 'Note', 'note' }, 'MiniHipatternsNote'),
            hex_color = hipatterns.gen_highlighter.hex_color(),
          },
        })
      end)
    '';
  }
  {
    plugin = mini-clue;
    type = "lua";
    config = /* lua */ ''
      -- Shows next-key hints on `<Leader>`, `\`, `[`/`]`, `` ` ``/`'`, `"`, `<C-w>`, `g`, `z`
      later(function()
        local miniclue = require('mini.clue')
        -- stylua: ignore
        miniclue.setup({
          clues = {
            leader_group_clues, -- from keymaps.nix
            miniclue.gen_clues.builtin_completion(),
            miniclue.gen_clues.g(),
            miniclue.gen_clues.marks(),
            miniclue.gen_clues.registers(),
            miniclue.gen_clues.windows({ submode_resize = true }),
            miniclue.gen_clues.z(),
          },
          triggers = {
            { mode = 'n', keys = '<Leader>' }, { mode = 'x', keys = '<Leader>' },
            { mode = 'n', keys = '\\' },
            { mode = 'n', keys = '[' }, { mode = 'n', keys = ']' },
            { mode = 'x', keys = '[' }, { mode = 'x', keys = ']' },
            { mode = 'i', keys = '<C-x>' },
            { mode = 'n', keys = 'g' }, { mode = 'x', keys = 'g' },
            { mode = 'n', keys = "'" }, { mode = 'n', keys = '`' },
            { mode = 'x', keys = "'" }, { mode = 'x', keys = '`' },
            { mode = 'n', keys = '"' }, { mode = 'x', keys = '"' },
            { mode = 'i', keys = '<C-r>' }, { mode = 'c', keys = '<C-r>' },
            { mode = 'n', keys = '<C-w>' },
            { mode = 'n', keys = 'z' }, { mode = 'x', keys = 'z' },
          },
        })
      end)
    '';
  }
]
