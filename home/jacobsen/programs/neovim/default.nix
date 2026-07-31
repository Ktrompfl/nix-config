{ pkgs, ... }:
{
  programs.neovim = {
    enable = true;
    viAlias = true;
    vimAlias = true;
    vimdiffAlias = true;
    defaultEditor = true;

    # disabled by default in home-manager state versions >= 26.05
    withPython3 = false;
    withRuby = false;

    extraPackages = with pkgs; [
      git
      ripgrep

      clang-tools
      harper # harper-ls: spelling/grammar
      lua-language-server
      markdown-oxide
      marksman
      nixd
      rust-analyzer
      ruff
      texlab
      ty # python type checker + language server
      yaml-language-server

      tinymist # also used by typst-preview
      websocat # typst-preview dependency

      python3Packages.jupytext # jupyter notebooks as plain text

      nixfmt
      prettier
      runic # julia formatter, see pkgs/runic.nix
      rustfmt
      shfmt
      stylua
      texlive.bin.latexindent
    ];

    initLua = (import ./options.nix) + (import ./keymaps.nix);

    plugins = import ./plugins { inherit pkgs; };
  };

  preservation.preserveAt.state-dir.directories = [ ".local/share/nvim" ];
}
