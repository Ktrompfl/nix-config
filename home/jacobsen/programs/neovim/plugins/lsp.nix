{ pkgs }:
with pkgs.vimPlugins;
[
  {
    plugin = nvim-lspconfig;
    type = "lua";
    config = /* lua */ ''
      vim.lsp.enable('clangd')

      -- Julia: JETLS (new) or classic LanguageServer.jl - both read from
      -- '~/.julia/environments/languageserver' (bootstrapped in julia.nix, also used
      -- by Claude Code's own LSP wiring in claude.nix). Flip to switch.
      local julia_use_jetls = true
      if julia_use_jetls then
        vim.lsp.config('jetls', {
          cmd = { vim.fn.expand('~/.julia/bin/jetls'), 'serve' },
          filetypes = { 'julia' },
          root_markers = { 'Project.toml' },
        })
        vim.lsp.enable('jetls')
      else
        vim.lsp.config('julials', {
          cmd = {
            'julia', '--startup-file=no', '--history-file=no', '--quiet',
            '--project=' .. vim.fn.expand('~/.julia/environments/languageserver'),
            '-e', 'using LanguageServer; runserver()',
          },
        })
        vim.lsp.enable('julials')
      end

      vim.lsp.enable('lua_ls')
      vim.lsp.enable('stylua')

      vim.lsp.enable('markdown_oxide')
      vim.lsp.enable('marksman')

      vim.lsp.config('nixd', {
        settings = {
          nixd = {
            nixpkgs = { expr = 'import <nixpkgs> { }' },
            formatting = { command = { 'nixfmt' } },
          },
        },
      })
      vim.lsp.enable('nixd')

      vim.lsp.enable('ty')
      vim.lsp.enable('ruff')

      vim.lsp.enable('rust_analyzer')
      vim.lsp.enable('texlab')

      vim.lsp.config('tinymist', {
        settings = { exportPdf = 'onSave', formatterMode = 'typstyle' },
      })
      vim.lsp.enable('tinymist')

      vim.lsp.enable('yamlls')

      -- Spelling/grammar for prose and code comments
      vim.lsp.config('harper_ls', { settings = { ['harper-ls'] = { dialect = 'British' } } })
      vim.lsp.enable('harper_ls')

      -- Code lens (inline "N references"/"Run test" annotations, e.g. rust-analyzer, JETLS).
      -- Not on by default; unlike inlay hints there's no vim.o toggle, only per-buffer enable.
      new_autocmd('LspAttach', nil, function(args)
        local client = vim.lsp.get_client_by_id(args.data.client_id)
        if client and client:supports_method('textDocument/codeLens') then
          vim.lsp.codelens.enable(true, { bufnr = args.buf })
        end
      end, 'Code lens enable')
    '';
  }
]
