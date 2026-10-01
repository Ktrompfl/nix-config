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
    # usbguard needs C++20, but still compiles with C++17;
    # remove once https://github.com/NixOS/nixpkgs/pull/568695 is available.
    usbguard = prev.usbguard.overrideAttrs (oldAttrs: {
      postPatch = (oldAttrs.postPatch or "") + ''
        substituteInPlace configure.ac \
          --replace-fail '-std=c++17' '-std=c++20'
      '';
    });

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

    # moonlight-qt 6.1.0 predates upstream's ffmpeg 7.1 API migration and no longer builds
    # against current ffmpeg. Follow master until 6.2.0 releases, as in
    # https://github.com/NixOS/nixpkgs/pull/552544
    # moonlight-qt = prev.moonlight-qt.overrideAttrs (oldAttrs: {
    #   version = "6.1.0-unstable-2026-08-06";

    #   src = final.fetchFromGitHub {
    #     owner = "moonlight-stream";
    #     repo = "moonlight-qt";
    #     rev = "2e13ed9977bc31c73caf8428f08f58d793313ece";
    #     hash = "sha256-kCm/YoFGcXhF/Abi5lRV5F7H1AbKJchdDOlfBVR0tRA=";
    #     fetchSubmodules = true;
    #   };

    #   # the Xcode < 14 fix is already in master
    #   patches = [ ];

    #   # would point at a tag that doesn't exist
    #   meta = removeAttrs oldAttrs.meta [ "changelog" ];
    # });
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
