{ pkgs }:
with pkgs.vimPlugins;
[
  {
    plugin = mini-deps;
    type = "lua";
    config = /* lua */ ''
      -- Plugin manager; Nix installs the plugins, this only provides staged `now`/`later`
      -- loading (plus graceful error handling) for startup performance.
      require('mini.deps').setup()
      local now, later = MiniDeps.now, MiniDeps.later
      -- Some setup only needs `now()` when started like `nvim path/to/file`
      local now_if_args = vim.fn.argc(-1) > 0 and now or later
    '';
  }
  {
    plugin = mini-basics;
    type = "lua";
    config = /* lua */ ''
      now(function()
        require('mini.basics').setup({
          options = { basic = false }, -- options are set in options.nix instead
          mappings = { windows = true, move_with_alt = true },
        })
      end)
    '';
  }
  {
    plugin = mini-icons;
    type = "lua";
    config = /* lua */ ''
      now(function() require('mini.icons').setup({ style = 'glyph' }) end)
    '';
  }
  {
    plugin = mini-misc;
    type = "lua";
    config = /* lua */ ''
      now_if_args(function()
        require('mini.misc').setup()
        MiniMisc.setup_auto_root() -- cd to nearest .git/Makefile ancestor
        MiniMisc.setup_restore_cursor()
        MiniMisc.setup_termbg_sync()
      end)
    '';
  }
  {
    plugin = mini-notify;
    type = "lua";
    config = /* lua */ ''
      now(function() require('mini.notify').setup() end)
    '';
  }
  {
    plugin = mini-sessions;
    type = "lua";
    config = /* lua */ ''
      now(function() require('mini.sessions').setup() end)
    '';
  }
  {
    plugin = mini-extra;
    type = "lua";
    config = /* lua */ ''
      -- Extra pickers/textobjects/highlighters used by other mini.* modules; must load
      -- before those (mini-edit.nix's mini.ai, mini-ui.nix's mini.hipatterns), so it lives
      -- here in mini-core.nix, which plugins/default.nix imports first.
      later(function() require('mini.extra').setup() end)
    '';
  }
]
