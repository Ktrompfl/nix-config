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

    flake-parts = {
      url = "github:hercules-ci/flake-parts";
      inputs.nixpkgs-lib.follows = "nixpkgs";
    };

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

    jail.url = "sourcehut:~alexdavid/jail.nix";

    jay = {
      url = "github:mahkoh/jay";
      inputs = {
        crane.follows = "crane";
        nixpkgs.follows = "nixpkgs";
        rust-overlay.follows = "rust-overlay";
      };
    };

    jay-screenshot = {
      url = "github:Ktrompfl/jay-screenshot";
      inputs = {
        jay.follows = "jay";
        nixpkgs.follows = "nixpkgs";
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
  };

  outputs =
    inputs@{ flake-parts, nixpkgs, ... }:
    let
      inherit (nixpkgs) lib;

      overlay = import ./overlays { inherit inputs; };
    in
    flake-parts.lib.mkFlake { inherit inputs; } (
      { withSystem, ... }:
      let
        mkHost =
          system: modules:
          withSystem system (
            { pkgs, ... }:
            lib.nixosSystem {
              modules = [
                ./home
                ./system
                { nixpkgs.pkgs = pkgs; }
              ]
              ++ modules;
              specialArgs = {
                inherit inputs;
                helpers = import ./lib { inherit lib pkgs; };
              };
            }
          );
      in
      {
        imports = [ inputs.git-hooks.flakeModule ];

        systems = import inputs.systems;

        perSystem =
          {
            config,
            pkgs,
            system,
            ...
          }:
          {
            _module.args.pkgs = import nixpkgs {
              inherit system;
              config.allowUnfree = true;
              overlays = [ overlay ];
            };

            packages = lib.filterAttrs (_: lib.isDerivation) (import ./pkgs { inherit inputs pkgs; });

            formatter = pkgs.nixfmt;

            pre-commit.settings.hooks = {
              nixfmt.enable = true;
              # rustfmt.enable = true;
              stylua.enable = true;
            };

            devShells = {
              default = config.pre-commit.devShell;

              rust =
                let
                  rustBin = inputs.rust-overlay.lib.mkRustBin { } pkgs;
                in
                pkgs.mkShell {
                  buildInputs = [
                    (rustBin.stable.latest.default.override {
                      extensions = [
                        "rust-src"
                        "rustfmt"
                        "clippy"
                      ];
                    })
                  ];
                };
            };
          };

        flake = {
          # Everything this flake adds to or changes about nixpkgs
          overlays.default = overlay;

          # Reusable nixos modules
          nixosModules.default = import ./modules/nixos;

          # Reusable hjem modules
          hjemModules.default = import ./modules/hjem;

          # NixOS configuration entrypoint
          nixosConfigurations = {
            # laptop
            luthadel = mkHost "x86_64-linux" [
              ./hosts/luthadel
              ./home/graphical.nix
            ];

            # desktop
            hallandren = mkHost "x86_64-linux" [
              ./hosts/hallandren
              ./home/graphical.nix
              ./home/gaming.nix
            ];
          };
        };
      }
    );
}
