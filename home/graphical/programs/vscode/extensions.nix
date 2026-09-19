{ pkgs }:
with pkgs.vscode-extensions;
[
  # if any extension is ever outdated again switch to  https://github.com/nix-community/nix-vscode-extensions
  dbaeumer.vscode-eslint
  eamodio.gitlens
  esbenp.prettier-vscode
  formulahendry.code-runner
  golang.go
  gruntfuggly.todo-tree
  james-yu.latex-workshop
  jnoortheen.nix-ide
  redhat.java
  redhat.vscode-xml
  redhat.vscode-yaml
  mechatroner.rainbow-csv
  mhutchie.git-graph
  ms-azuretools.vscode-docker
  ms-python.black-formatter # python formatter
  ms-python.python
  ms-toolsai.jupyter
  ms-vscode.cpptools
  myriad-dreamin.tinymist # combined typst lsp and preview
  naumovs.color-highlight
  streetsidesoftware.code-spell-checker
  sumneko.lua
  tamasfe.even-better-toml
  usernamehw.errorlens
  valentjn.vscode-ltex
]
++ pkgs.vscode-utils.extensionsFromVscodeMarketplace [
  # not packaged in nixpkgs' vscode-extensions set
  {
    name = "jetls-client";
    publisher = "aviatesk";
    version = "0.5.0";
    sha256 = "1cbxli2i41irszjyms6759v3fj9vvvhd28q5n4l8g5p0b33mj2nm";
  }
]
