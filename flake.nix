{
  description = "Personal NixOS configurations";

  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-unstable";
    systems.url = "github:nix-systems/default-linux";

    crane.url = "github:ipetkov/crane";

    disko = {
      url = "github:nix-community/disko";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    flake-compat = {
      url = "github:NixOS/flake-compat";
      flake = false;
    };

    flake-parts.url = "github:hercules-ci/flake-parts";

    git-hooks = {
      url = "github:cachix/git-hooks.nix";
      inputs = {
        flake-compat.follows = "flake-compat";
        nixpkgs.follows = "nixpkgs";
      };
    };

    hjem = {
      url = "github:feel-co/hjem";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    jay = {
      url = "github:mahkoh/jay";
      inputs = {
        crane.follows = "crane";
        nixpkgs.follows = "nixpkgs";
        rust-overlay.follows = "rust-overlay";
      };
    };

    # omitting follows costs a second nixpkgs evaluation but guarantees binary cache hits
    llm-agents.url = "github:numtide/llm-agents.nix";

    nix-mineral = {
      url = "github:cynicsketch/nix-mineral";
      inputs = {
        flake-compat.follows = "flake-compat";
        flake-parts.follows = "flake-parts";
        nixpkgs.follows = "nixpkgs";
      };
    };

    nur = {
      url = "github:nix-community/NUR";
      inputs = {
        flake-parts.follows = "flake-parts";
        nixpkgs.follows = "nixpkgs";
      };
    };

    preservation.url = "github:nix-community/preservation";

    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    sops-nix = {
      url = "github:Mic92/sops-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };

    spicetify-nix = {
      url = "github:Gerg-L/spicetify-nix";
      inputs = {
        nixpkgs.follows = "nixpkgs";
        systems.follows = "systems";
      };
    };

    zed-julia = {
      url = "github:JuliaEditorSupport/zed-julia";
      flake = false;
    };
  };

  outputs =
    inputs@{
      self,
      nixpkgs,
      systems,
      ...
    }:
    let
      lib = nixpkgs.lib.extend (
        final: prev: {
          generators = prev.generators // import ./lib/generators.nix { lib = final; };
        }
      );
      eachSystem = lib.genAttrs (import systems);

      # nixpkgs with this flake's overlays applied. The `packages` output goes
      # through it so that `nix build .#foo` and the hosts, which apply the
      # same overlay in ./system, cannot disagree about what `foo` is.
      pkgsFor =
        system:
        import nixpkgs {
          inherit system;
          config.allowUnfree = true;
          overlays = [ self.overlays.default ];
        };
    in
    {
      # Run the hooks in a sandbox with 'nix flake check'.
      # Read-only filesystem and no internet access.
      checks = eachSystem (system: {
        pre-commit-check = inputs.git-hooks.lib.${system}.run {
          src = ./.;
          hooks = {
            nixfmt.enable = true;
            # rustfmt.enable = true;
            stylua.enable = true;
          };
        };
      });

      # Enter a development shell with 'nix develop'.
      # The hooks will be installed automatically.
      devShells = eachSystem (system: {
        default =
          let
            pkgs = nixpkgs.legacyPackages.${system};
            inherit (self.checks.${system}.pre-commit-check) shellHook enabledPackages;
          in
          pkgs.mkShell {
            inherit shellHook;
            buildInputs = enabledPackages;
          };

        rust =
          let
            pkgs = nixpkgs.legacyPackages.${system};
            rustBin = inputs.rust-overlay.lib.mkRustBin { } pkgs;
            rustToolchain = rustBin.stable.latest.default.override {
              extensions = [
                "rust-src"
                "rustfmt"
                "clippy"
              ];
            };
          in
          pkgs.mkShell {
            buildInputs = [ rustToolchain ];
          };
      });

      # Formatter for your nix files, available through 'nix fmt'
      formatter = eachSystem (system: nixpkgs.legacyPackages.${system}.nixfmt);

      # Your custom packages
      # Accessible through 'nix build', 'nix shell', etc
      packages = eachSystem (
        system:
        import ./pkgs {
          inherit inputs;
          pkgs = pkgsFor system;
        }
      );

      # Everything this flake adds to or changes about nixpkgs
      overlays.default = import ./overlays { inherit inputs; };

      # Reusable nixos modules you might want to export
      # These are usually stuff you would upstream into nixpkgs
      nixosModules.default = import ./modules/nixos;

      # Reusable hjem modules
      hjemModules.default = import ./modules/hjem;

      # NixOS configuration entrypoint
      # Available through 'nixos-rebuild --flake .#your-hostname'
      nixosConfigurations =
        let
          base = [
            ./home
            ./system
          ];
          graphical = [ ./home/graphical.nix ];
          gaming = [ ./home/gaming.nix ];

          mkHost =
            modules:
            lib.nixosSystem {
              inherit modules;
              specialArgs = { inherit inputs; };
            };
        in
        {
          # laptop
          luthadel = mkHost (base ++ graphical ++ [ ./hosts/luthadel ]);

          # desktop
          hallandren = mkHost (base ++ graphical ++ gaming ++ [ ./hosts/hallandren ]);
        };
    };
}
