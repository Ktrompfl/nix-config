{
  config,
  inputs,
  lib,
  pkgs,
  ...
}:
{
  imports = [
    inputs.self.nixosModules
    inputs.carrot.nixosModules.default
    inputs.jay.nixosModules.default

    ./boot
    ./hardware
    ./network
    ./programs
    ./services

    ./localization.nix
    ./packages.nix
    ./preservation.nix
    ./security.nix
    ./sops.nix
    ./stylix.nix
    ./users.nix
  ];

  documentation = {
    doc.enable = false;
    info.enable = false;
    nixos.enable = false;
  };

  nixpkgs = {
    config.allowUnfree = true;

    overlays = [
      # Add overlays this flake exports (from overlays and pkgs dir):
      inputs.self.overlays.additions
      inputs.self.overlays.modifications

      # Add overlays from other flakes
      inputs.nur.overlays.default

      # Make supported packages use lix instead of nix
      (final: prev: {
        inherit (prev.lixPackageSets.stable)
          nixpkgs-review
          nix-eval-jobs
          nix-fast-build
          colmena
          ;
      })
    ];
  };

  nix =
    let
      flakeInputs = lib.filterAttrs (_: v: lib.isType "flake" v) inputs;
    in
    {
      package = pkgs.lixPackageSets.stable.lix;

      # pin the registry to avoid downloading and evaling a new nixpkgs version every time
      registry = lib.mapAttrs (_: v: { flake = v; }) flakeInputs;

      # set the path for channels compat
      nixPath = lib.mapAttrsToList (key: _: "${key}=flake:${key}") config.nix.registry;

      # disable channels
      channel.enable = false;

      optimise = {
        automatic = true;
        dates = [ "weekly" ];
      };

      settings = {
        connect-timeout = 5;
        fallback = true;
        http-connections = 50;

        builders-use-substitutes = true;
        warn-dirty = true;
        experimental-features = [
          "nix-command"
          "flakes"
        ];
        flake-registry = "/etc/nix/registry.json";

        # for direnv GC roots
        keep-derivations = true;
        keep-outputs = true;

        trusted-users = [
          "root"
          "@wheel"
        ];

        accept-flake-config = false;

        substituters = [
          # high priority since it's almost always used
          "https://cache.nixos.org?priority=10"
          "https://nix-community.cachix.org"
          "https://cache.numtide.com"
        ];

        trusted-public-keys = [
          "cache.nixos.org-1:6NCHdD59X431o0gWypbMrAURkbJ16ZPMQFGspcDShjY="
          "nix-community.cachix.org-1:mB9FSh9qf2dCimDSUo8Zy7bkq5CX+/rkCWyvRCYg3Fs="
          "niks3.numtide.com-1:DTx8wZduET09hRmMtKdQDxNNthLQETkc/yaX7M4qK0g="
        ];
      };
    };

  systemd.timers.nix-optimise = {
    unitConfig = {
      ConditionACPower = true;
      IOSchedulingClass = "idle";
      CPUSchedulingPolicy = "idle";
    };
    timerConfig = {
      Persistent = lib.mkForce false;
      RandomizedDelaySec = lib.mkForce "2h";
    };
  };
}
