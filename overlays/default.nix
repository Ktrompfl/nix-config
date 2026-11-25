{ inputs, ... }:
let
  inherit (inputs.nixpkgs.lib) composeManyExtensions;

  additions =
    final: _prev:
    import ../pkgs {
      pkgs = final;
      inherit inputs;
    };

  modifications = final: prev: {
    # zotero no longer builds on firefox-esr-153, and firefox-esr-140 is gone;
    # remove once https://github.com/NixOS/nixpkgs/issues/568692 is fixed.
    inherit (inputs.nixpkgs-zotero.legacyPackages.${final.stdenv.hostPlatform.system}) zotero;

    # wl-mirror allocates its capture buffer in the first dmabuf format the
    # compositor offers; jay offers formats in random order, including some
    # (e.g. RGB888, RGBA4444) that gbm cannot allocate on amdgpu.
    wl-mirror = prev.wl-mirror.overrideAttrs (oldAttrs: {
      patches = (oldAttrs.patches or [ ]) ++ [ ./wl-mirror-prefer-rgb8888.patch ];
    });

    # fuzzel truncates the scaled border width, so a 1px border vanishes on
    # outputs with a fractional scale below 1.
    fuzzel = prev.fuzzel.overrideAttrs (oldAttrs: {
      patches = (oldAttrs.patches or [ ]) ++ [ ./fuzzel-round-border-size.patch ];
    });

    # btop 1.4.8 redraws the cpu box on proc box input even when it is hidden,
    # constructing a graph with negative height that spins forever;
    # go back to the nixpkgs release once it includes 00cbab2.
    btop = prev.btop.overrideAttrs (_: {
      version = "1.4.8-unstable-2026-11-09";
      src = final.fetchFromGitHub {
        owner = "aristocratos";
        repo = "btop";
        rev = "00cbab2dba28bf5b0b33d5f84af6a77c21b1d61d";
        hash = "sha256-57u4QMeXN+ESpeQof9iN/rVi2Hc1Y6JwOcygkPHj1bc=";
      };
      preVersionCheck = "version=1.4.8";
    });
  };

  # Make supported packages use lix instead of nix.
  lix = _final: prev: {
    inherit (prev.lixPackageSets.stable)
      nixpkgs-review
      nix-eval-jobs
      nix-fast-build
      colmena
      ;
  };
in
composeManyExtensions [
  additions
  modifications
  lix

  inputs.jay.overlays.default
  inputs.jay-screenshot.overlays.default
  inputs.nix-vscode-extensions.overlays.default
  inputs.nur.overlays.default
]
