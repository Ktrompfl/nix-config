{ config, pkgs, ... }:
{
  environment.sessionVariables = {
    TEXMFHOME = "${config.xdg.data.directory}/texmf";
    TEXMFCONFIG = "${config.xdg.state.directory}/texlive/texmf-config";
    TEXMFVAR = "${config.xdg.cache.directory}/texlive/texmf-var";
  };

  packages = [
    (pkgs.texlive.withPackages (
      tpkgs: with tpkgs; [
        collection-basic
        collection-binextra
        collection-fontsrecommended
        collection-fontutils
        collection-langenglish
        collection-langgerman
        collection-latex
        collection-latexrecommended
        collection-luatex
        collection-mathscience
        collection-metapost
        collection-plaingeneric
        minitoc
      ]
    ))
  ];
}
