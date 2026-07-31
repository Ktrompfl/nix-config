{ pkgs }:
with pkgs.vimPlugins;
[
  {
    # only install selected grammars to avoid building all grammars from source
    plugin = nvim-treesitter.withPlugins (
      p: with p; [
        bash
        c
        cmake
        comment
        cpp
        css
        csv
        diff
        dockerfile
        gitcommit
        gitignore
        haskell
        html
        ini
        javascript
        json
        julia
        latex
        lua
        make
        markdown
        markdown_inline
        nginx
        nix
        python
        query
        regex
        rst
        rust
        sql
        toml
        typescript
        typst
        vim
        vimdoc
        xml
        yaml
      ]
    );
    type = "lua";
    config = /* lua */ ''
      vim.api.nvim_create_autocmd('FileType', {
        group = vim.api.nvim_create_augroup('treesitter.setup', {}),
        callback = function(args)
          local language = vim.treesitter.language.get_lang(args.match) or args.match
          if vim.treesitter.language.add(language) then
            vim.wo.foldexpr = 'v:lua.vim.treesitter.foldexpr()'
            vim.wo.foldmethod = 'expr'
            vim.treesitter.start(args.buf, language)
            vim.bo[args.buf].indentexpr = "v:lua.require'nvim-treesitter'.indentexpr()"
          end
        end,
      })
    '';
  }
]
