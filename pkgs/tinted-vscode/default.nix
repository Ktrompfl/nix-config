{
  lib,
  fetchFromGitHub,
  vscode-utils,
  writeText,
}:
let
  src = fetchFromGitHub {
    owner = "tinted-theming";
    repo = "tinted-vscode";
    rev = "9e9de5fa4e8800e09371dc0b76ccf2142697283d";
    hash = "sha256-j+3oTO6t8rPT7RbCgXdl8gQKONmGtD98UIVMue8OG5w=";
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

  # The name to set as `workbench.colorTheme`.
  themeName = palette: "Tinted ${palette.meta.name or "Unnamed"}";
in
{
  inherit src themeName;

  # A one-theme extension; the upstream one ships every scheme instead of ours.
  extensionFor =
    palette:
    let
      system = palette.meta.system or "base16";
      template = "${src}/templates/${system}.mustache";
      slug = palette.meta.slug or "tinted";

      theme = writeText "vscode-theme-${slug}.json" (render template palette);

      manifest = writeText "package.json" (
        builtins.toJSON {
          name = "tinted";
          publisher = "local";
          version = "0.0.0";
          engines.vscode = "*";
          contributes.themes = [
            {
              label = themeName palette;
              uiTheme = if (palette.meta.variant or "dark") == "light" then "vs" else "vs-dark";
              path = "./theme.json";
            }
          ];
        }
      );
    in
    assert lib.assertMsg (builtins.pathExists template)
      "tinted-vscode has no template for scheme system '${system}'";
    vscode-utils.buildVscodeExtension {
      pname = "tinted-${slug}";
      version = "0.0.0";
      vscodeExtPublisher = "local";
      vscodeExtName = "tinted";
      vscodeExtUniqueId = "local.tinted";

      dontUnpack = true;
      installPhase = ''
        runHook preInstall
        install -Dm644 ${manifest} "$out/$installPrefix/package.json"
        install -Dm644 ${theme} "$out/$installPrefix/theme.json"
        runHook postInstall
      '';
    };
}
