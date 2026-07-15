{ pkgs, ... }:
{
  # note: the home-manager module uses deprecated options to combine the texlive package
  home.packages = [
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
        # extra packages
        minitoc
      ]
    ))
  ];
}
