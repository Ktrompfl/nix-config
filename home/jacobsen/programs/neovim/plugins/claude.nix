{ pkgs }:
with pkgs.vimPlugins;
[
  {
    plugin = claudecode-nvim;
    type = "lua";
    config = /* lua */ ''
      -- Starts a local server any `claude` CLI - including one in another terminal - auto-discovers,
      -- so selections/diffs/`@file` mentions work either way. 'native' avoids needing snacks.nvim.
      later(function()
        require('claudecode').setup({ terminal = { provider = 'native' } })
      end)
    '';
  }
]
