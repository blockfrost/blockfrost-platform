{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.05";
    flake-parts.url = "github:hercules-ci/flake-parts";
    flake-compat = {
      url = "github:edolstra/flake-compat";
      flake = false;
    };
    treefmt-nix = {
      url = "github:numtide/treefmt-nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    crane.url = "github:ipetkov/crane";
    fenix = {
      url = "github:nix-community/fenix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    cardano-node = {
      # Temporarily for `cardano-cli`, for `hydra-node`:
      url = "github:IntersectMBO/cardano-node/10.5.3";
      flake = false; # otherwise, +2k dependencies we don’t really use
    };
    hydra = {
      url = "github:cardano-scaling/hydra/1.0.0";
      flake = false;
    };
    devshell = {
      url = "github:numtide/devshell";
      inputs.nixpkgs.follows = "nixpkgs";
    };
    advisory-db = {
      url = "github:rustsec/advisory-db";
      flake = false;
    };
  };

  outputs = inputs: let
    inherit (inputs.nixpkgs) lib;
  in
    inputs.flake-parts.lib.mkFlake {inherit inputs;} ({config, ...}: {
      imports = [
        inputs.devshell.flakeModule
        inputs.treefmt-nix.flakeModule
      ];

      systems = [
        "x86_64-linux"
        "aarch64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];
      perSystem = {system, ...}: let
        internal = inputs.self.internal.${system};
      in {
        packages.default = internal.package;

        devshells.default = import ./nix/devshells.nix {inherit inputs;};

        checks = internal.cargoChecks // internal.nixChecks;

        treefmt = {pkgs, ...}: {
          projectRootFile = "flake.nix";
          programs = {
            alejandra.enable = true; # Nix
            prettier.enable = true;
            rustfmt.enable = true;
            rustfmt.package = internal.rustPackages.rustfmt;
            yamlfmt.enable = pkgs.system != "x86_64-darwin"; # a treefmt-nix+yamlfmt bug on Intel Macs
            taplo.enable = true; # TOML
            shfmt.enable = true;
          };
          settings.formatter.rustfmt.options = [
            "--config-path"
            (builtins.path {
              name = "rustfmt.toml";
              path = ./rustfmt.toml;
            })
          ];
        };
      };

      flake = {
        internal = lib.genAttrs config.systems (
          targetSystem: import ./nix/internal/unix.nix {inherit inputs targetSystem;}
        );

        hydraJobs = let
          allJobs = {
            blockfrost-gateway = lib.genAttrs config.systems (
              targetSystem: inputs.self.internal.${targetSystem}.package
            );
            devshell = lib.genAttrs config.systems (
              targetSystem: inputs.self.devShells.${targetSystem}.default
            );
            inherit (inputs.self) checks;
          };
        in
          allJobs
          // {
            required = inputs.nixpkgs.legacyPackages.x86_64-linux.releaseTools.aggregate {
              name = "github-required";
              meta.description = "All jobs required to pass CI";
              constituents = lib.collect lib.isDerivation allJobs;
            };
          };

        nixConfig = {
          extra-substituters = ["https://cache.iog.io"];
          extra-trusted-public-keys = ["hydra.iohk.io:f/Ea+s+dFdN+3Y/G+FDgSq+a5NEWhJGzdjvKNGv0/EQ="];
          allow-import-from-derivation = "true";
        };
      };
    });
}
