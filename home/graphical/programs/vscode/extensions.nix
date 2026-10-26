{ pkgs, palette }:
let
  # Latest marketplace releases that support the packaged vscode, updated with
  # the nix-vscode-extensions input.
  marketplace =
    (pkgs.nix-vscode-extensions.forVSCodeVersion pkgs.vscode.version).vscode-marketplace-release;
in
with marketplace;
[
  (pkgs.tinted-vscode.extensionFor palette)

  # extension packs are not resolved, so their members are listed explicitly

  # python, with ty in place of pylance like zed
  astral-sh.ty
  charliermarsh.ruff
  ms-python.debugpy
  ms-python.python

  # julia
  aviatesk.jetls-client

  # notebooks
  ms-toolsai.jupyter
  ms-toolsai.jupyter-renderers

  # typst
  myriad-dreamin.tinymist

  # nix
  jnoortheen.nix-ide
  mkhl.direnv

  # data and configuration files
  mechatroner.rainbow-csv
  tamasfe.even-better-toml

  # spelling and grammar, and diagnostics next to the code
  elijah-potter.harper
  usernamehw.errorlens
]
