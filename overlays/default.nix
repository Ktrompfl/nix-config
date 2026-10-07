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

    # With julia 1.13, Pkg looks up a name for every UUID in a registry's
    # Deps.toml, including the dependencies of versions the minimal registry
    # dropped (SnoopPrecompile for Parsers 2.5), and withPackages fails.
    julia = prev.julia.overrideAttrs (oldAttrs: {
      passthru = oldAttrs.passthru // {
        withPackages =
          let
            src = final.applyPatches {
              name = "julia-modules";
              src = "${inputs.nixpkgs}/pkgs/development/julia-modules";
              patches = [ ./julia-minimal-registry.patch ];
            };
          in
          final.callPackage src { julia = final.julia; };
      };
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
  inputs.nur.overlays.default
]
