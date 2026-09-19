{
  config,
  lib,
  pkgs,
  ...
}:
let
  plugins = with pkgs.vimPlugins; {
    inherit
      mini-deps
      mini-basics
      mini-icons
      mini-misc
      mini-notify
      mini-sessions
      mini-extra
      mini-statusline
      mini-tabline
      mini-cursorword
      mini-indentscope
      mini-hipatterns
      mini-clue
      mini-ai
      mini-align
      mini-bracketed
      mini-bufremove
      mini-comment
      mini-completion
      mini-jump2d
      mini-keymap
      mini-move
      mini-operators
      mini-pairs
      mini-splitjoin
      mini-surround
      mini-trailspace
      mini-diff
      mini-files
      mini-git
      mini-pick
      mini-visits
      nvim-lspconfig
      conform-nvim
      jupytext-nvim
      render-markdown-nvim
      typst-preview-nvim
      claudecode-nvim
      ;

    # only install selected grammars to avoid building all grammars from source
    nvim-treesitter = nvim-treesitter.withPlugins (
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
        rust
        ssh_config
        toml
        typescript
        typst
        vim
        vimdoc
        xml
        yaml
      ]
    );
  };

  colorscheme = /* lua */ ''
    require('mini.base16').setup({
      palette = {
    ${lib.concatStringsSep "\n" (
      map (slot: "    ${slot} = '${config.theme.colors.withHashtag.${slot}}',") [
        "base00"
        "base01"
        "base02"
        "base03"
        "base04"
        "base05"
        "base06"
        "base07"
        "base08"
        "base09"
        "base0A"
        "base0B"
        "base0C"
        "base0D"
        "base0E"
        "base0F"
      ]
    )}
      }
    })
  '';
in
{
  packages = with pkgs; [
    neovim

    # tools the configuration shells out to
    git
    ripgrep
  ];

  xdg.config.files = {
    "nvim/init.lua".text = /* lua */ ''
      -- Generated. The configuration itself is in lua/; only the palette below
      -- comes from Nix.
      require('options')
      require('keymaps')

      ${colorscheme}

      require('plugins')
    '';

    "nvim/lua/options.lua".source = ./lua/options.lua;
    "nvim/lua/keymaps.lua".source = ./lua/keymaps.lua;
    "nvim/lua/plugins.lua".source = ./lua/plugins.lua;
  }
  // lib.mapAttrs' (
    name: drv: lib.nameValuePair "nvim/pack/plugins/start/${name}" { source = drv; }
  ) plugins;

  preservation.preserveAt.state-dir.directories = [ ".local/share/nvim" ];
}
