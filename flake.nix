{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixos-25.05";
    flake-parts.url = "github:hercules-ci/flake-parts";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";
    crane.url = "github:ipetkov/crane";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
    flake-compat.url = "github:input-output-hk/flake-compat";
    flake-compat.flake = false;
    cardano-node.url = "github:IntersectMBO/cardano-node/10.5.1";
    cardano-node.flake = false; # otherwise, +2k dependencies we don’t really use
    dolos.url = "github:txpipe/dolos/v1.0.0-beta.5";
    dolos.flake = false;
    blockfrost-tests.url = "github:blockfrost/blockfrost-tests/f6a742f2046e80139c1ddbc519ed3750253de8e6";
    blockfrost-tests.flake = false;
    mithril.url = "github:input-output-hk/mithril/2524.0";
    testgen-hs.url = "github:input-output-hk/testgen-hs/10.4.1.1"; # make sure it follows cardano-node
    testgen-hs.flake = false; # otherwise, +2k dependencies we don’t really use
    devshell.url = "github:numtide/devshell";
    devshell.inputs.nixpkgs.follows = "nixpkgs";
    cardano-playground.url = "github:input-output-hk/cardano-playground/56ebfef5595c43014029b039ade01b0ef06233e0";
    cardano-playground.flake = false; # otherwise, +9k dependencies in flake.lock…
    advisory-db.url = "github:rustsec/advisory-db";
    advisory-db.flake = false;
    nixpkgs-nsis.url = "github:input-output-hk/nixpkgs/be445a9074f139d63e704fa82610d25456562c3d";
    nixpkgs-nsis.flake = false;
    nix-bundle-exe.url = "github:3noch/nix-bundle-exe";
    nix-bundle-exe.flake = false;
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
        packages =
          {
            default = internal.package;
            blockfrost-platform = internal.package;
            inherit (internal) tx-build cardano-address testgen-hs;
          }
          // (lib.optionalAttrs (system == "x86_64-linux") {
            blockfrost-platform-x86_64-windows = inputs.self.internal.x86_64-windows.package;
          });

        devshells.default = import ./nix/devshells.nix {inherit inputs;};

        checks = internal.cargoChecks // internal.nixChecks;

        treefmt = {pkgs, ...}: {
          projectRootFile = "flake.nix";
          programs = {
            alejandra.enable = true; # Nix
            prettier.enable = true;
            rufo.enable = true; # Ruby
            rustfmt.enable = true;
            rustfmt.package = internal.rustPackages.rustfmt;
            shfmt.enable = true;
            taplo.enable = true; # TOML
            yamlfmt.enable = pkgs.system != "x86_64-darwin"; # a treefmt-nix+yamlfmt bug on Intel Macs
          };
          settings.global.excludes = [
            "**/.eslintignore"
            "**/.gitignore"
            "**/.gitkeep"
            "**/.prettierrc"
            "**/.yarnrc"
            "*.diff"
            "*.nsi"
            "*.png"
            "*.svg"
            "*.xml"
            "*.zip"
            ".editorconfig"
            "Dockerfile"
            "LICENSE"
            "target/**/*"
          ];
          settings.formatter = {
            prettier.options = [
              "--config"
              (builtins.path {
                path = ./docs/.prettierrc;
                name = "prettierrc.json";
              })
            ];
            rustfmt.options = [
              "--config-path"
              (builtins.path {
                name = "rustfmt.toml";
                path = ./rustfmt.toml;
              })
            ];
          };
        };
      };

      flake = {
        internal =
          lib.genAttrs config.systems (
            targetSystem: import ./nix/internal/unix.nix {inherit inputs targetSystem;}
          )
          // lib.genAttrs ["x86_64-windows"] (
            targetSystem: import ./nix/internal/windows.nix {inherit inputs targetSystem;}
          );

        nixosModule = {
          pkgs,
          lib,
          ...
        }: {
          imports = [./nix/nixos];
          services.blockfrost-platform.package = lib.mkDefault inputs.self.packages.${pkgs.system}.blockfrost-platform;
        };

        hydraJobs = let
          crossSystems = ["x86_64-windows"];
          allJobs = {
            blockfrost-platform = lib.genAttrs (config.systems ++ crossSystems) (
              targetSystem: inputs.self.internal.${targetSystem}.package
            );
            devshell = lib.genAttrs config.systems (
              targetSystem: inputs.self.devShells.${targetSystem}.default
            );
            archive = lib.genAttrs (config.systems ++ crossSystems) (
              targetSystem: inputs.self.internal.${targetSystem}.archive
            );
            installer = {
              x86_64-windows = inputs.self.internal.x86_64-windows.installer;
              x86_64-darwin = inputs.self.internal.x86_64-darwin.installer;
              aarch64-darwin = inputs.self.internal.aarch64-darwin.installer;
            };
            homebrew-tap = {
              aarch64-darwin = inputs.self.internal.aarch64-darwin.homebrew-tap;
            };
            curl-bash-install = {
              x86_64-linux = inputs.self.internal.x86_64-linux.curl-bash-install;
            };
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
