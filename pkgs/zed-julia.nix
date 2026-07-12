{
  pkgs,
  stdenv,
  inputs,
  fetchFromGitHub,
  fetchurl,
  autoPatchelfHook,
  runCommand,
}:
let
  src = inputs.zed-julia;

  rustPkgs = pkgs.extend inputs.rust-overlay.overlays.default;
  rustToolchain = rustPkgs.rust-bin.stable.latest.default.override {
    targets = [ "wasm32-wasip2" ];
  };
  craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;

  cargoArtifacts = craneLib.buildDepsOnly {
    inherit src;
    CARGO_BUILD_TARGET = "wasm32-wasip2";
    doCheck = false;
  };

  extensionWasm = craneLib.buildPackage {
    inherit src cargoArtifacts;
    CARGO_BUILD_TARGET = "wasm32-wasip2";
    doCheck = false;
    installPhaseCommand = ''
      mkdir -p $out
      cp target/wasm32-wasip2/release/zed_julia.wasm $out/extension.wasm
    '';
  };

  # Pinned to the grammar commit referenced by extension.toml's [grammars.julia];
  # bump alongside it if upstream changes the commit.
  tree-sitter-julia = fetchFromGitHub {
    owner = "tree-sitter";
    repo = "tree-sitter-julia";
    rev = "e0f9dcd180fdcfcfa8d79a3531e11d99e79321d3";
    hash = "sha256-Jk2jby7vWWSdnUU8s8zIIfyXFt7keWPJPyTyxPBrqBw=";
  };

  # Zed compiles grammars with a wasi-sdk clang. Its bundled wasi-libc is built
  # single-threaded; nixpkgs' pkgsCross.wasi32 toolchain is thread-enabled and
  # emits an extra `__wasi_init_tp` import that tree-sitter's minimal wasm
  # loader (lib/src/wasm_store.c) doesn't provide, so use the real thing.
  wasiSdk = stdenv.mkDerivation {
    pname = "wasi-sdk";
    version = "25.0";
    src = fetchurl {
      url = "https://github.com/WebAssembly/wasi-sdk/releases/download/wasi-sdk-25/wasi-sdk-25.0-x86_64-linux.tar.gz";
      hash = "sha256-UmQN3hNZm/EnqVSZ5h1tZAJWEZRW0a+Il6tnJbzz2Jw=";
    };
    nativeBuildInputs = [ autoPatchelfHook ];
    buildInputs = [ stdenv.cc.cc.lib ];
    dontConfigure = true;
    dontBuild = true;
    installPhase = ''
      mkdir -p $out
      cp -r . $out
    '';
  };

  grammarWasm = stdenv.mkDerivation {
    pname = "zed-julia-grammar";
    version = tree-sitter-julia.rev;
    src = tree-sitter-julia;
    dontConfigure = true;
    buildPhase = ''
      runHook preBuild
      ${wasiSdk}/bin/clang --sysroot=${wasiSdk}/share/wasi-sysroot \
        -fPIC -shared -Os -Wl,--export=tree_sitter_julia \
        -o julia.wasm -I src src/parser.c src/scanner.c
      runHook postBuild
    '';
    installPhase = ''
      mkdir -p $out
      cp julia.wasm $out/julia.wasm
    '';
  };
in
runCommand "zed-julia-extension" { } ''
  mkdir -p $out/grammars $out/languages
  cp -r ${src}/languages/. $out/languages/
  cp ${grammarWasm}/julia.wasm $out/grammars/julia.wasm
  cp ${extensionWasm}/extension.wasm $out/extension.wasm

  # extension.toml as published doesn't populate `languages` (Zed fills that in
  # at build time); insert it before the first [table] so the dev install works.
  awk '/^\[/ && !done { print "languages = [\"languages/julia\"]"; print ""; done=1 } { print }' \
    ${src}/extension.toml > $out/extension.toml
''
