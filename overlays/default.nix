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
