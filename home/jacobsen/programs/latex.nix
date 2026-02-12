{
  # latex via texlive
  programs.texlive = {
    enable = true;
    # add all required texlive packages here
    extraPackages = tpkgs: {
      inherit (tpkgs)
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
        ;
    };
  };
}
