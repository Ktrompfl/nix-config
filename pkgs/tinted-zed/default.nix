{
  lib,
  fetchFromGitHub,
  writeText,
}:
let
  src = fetchFromGitHub {
    owner = "tinted-theming";
    repo = "tinted-zed";
    rev = "c0067831e51df5f3cee6ba0c0fbcc598860379db";
    hash = "sha256-E6l2DbamdAMIzhLlYQbsOBkAUnLiYnbAG5TbGnersWQ=";
  };

  variables =
    palette:
    lib.mapAttrs' (slot: hex: lib.nameValuePair "${slot}-hex" hex) palette.slots
    // {
      scheme-name = palette.meta.name or "Unnamed";
      scheme-author = palette.meta.author or "";
      scheme-variant = palette.meta.variant or "dark";
      scheme-slug = palette.meta.slug or "unnamed";
      scheme-system = palette.meta.system or "base16";
    };

  escape =
    builtins.replaceStrings
      [ "&" "<" ">" ''"'' "'" ]
      [ "&amp;" "&lt;" "&gt;" "&quot;" "&#39;" ];

  render =
    template: palette:
    let
      vars = variables palette;
      names = lib.attrNames vars;
    in
    builtins.replaceStrings (map (n: "{{${n}}}") names) (map (n: escape vars.${n}) names) (
      builtins.readFile template
    );
in
{
  inherit src;

  themeFor =
    palette:
    let
      system = palette.meta.system or "base16";
      template = "${src}/templates/${system}.mustache";
    in
    assert lib.assertMsg (builtins.pathExists template)
      "tinted-zed has no template for scheme system '${system}'";
    writeText "zed-theme-${palette.meta.slug or "tinted"}.json" (render template palette);
}
