{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";
    crane.url = "github:ipetkov/crane";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
    flake-compat.url = "github:input-output-hk/flake-compat";
    flake-compat.flake = false;
    cardano-node.url = "github:IntersectMBO/cardano-node/10.1.4";
    cardano-node.flake = false; # otherwise, +2k dependencies we don’t really use
    testgen-hs.url = "github:input-output-hk/testgen-hs/10.1.4.1"; # make sure it follows cardano-node
    testgen-hs.flake = false; # otherwise, +2k dependencies we don’t really use
    devshell.url = "github:numtide/devshell";
    devshell.inputs.nixpkgs.follows = "nixpkgs";
    cardano-playground.url = "github:input-output-hk/cardano-playground/39ea4db0daa11d6334a55353f685e185765a619b";
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

      flake.internal =
        lib.genAttrs config.systems (
          targetSystem: import ./nix/internal/unix.nix {inherit inputs targetSystem;}
        )
        // lib.genAttrs ["x86_64-windows"] (
          targetSystem: import ./nix/internal/windows.nix {inherit inputs targetSystem;}
        );

      systems = [
        "x86_64-linux"
        # "aarch64-linux"
        "aarch64-darwin"
        "x86_64-darwin"
      ];
      perSystem = {
        config,
        system,
        pkgs,
        ...
      }: let
        internal = inputs.self.internal.${system};
      in {
        packages =
          {
            default = internal.package;
            inherit (internal) tx-build cardano-address testgen-hs;
          }
          // (lib.optionalAttrs (system == "x86_64-linux") {
            default-x86_64-windows = inputs.self.internal.x86_64-windows.package;
          });

        devshells.default = import ./nix/devshells.nix {inherit inputs;};

        checks = internal.cargoChecks;

        treefmt = {pkgs, ...}: {
          projectRootFile = "flake.nix";
          programs.alejandra.enable = true; # Nix
          programs.prettier.enable = true;
          settings.formatter.prettier.options = [
            "--config"
            (builtins.path {
              path = ./docs/.prettierrc;
              name = "prettierrc.json";
            })
          ];
          programs.rufo.enable = true; # Ruby
          programs.rustfmt.enable = true;
          programs.yamlfmt.enable = pkgs.system != "x86_64-darwin"; # a treefmt-nix+yamlfmt bug on Intel Macs
          programs.taplo.enable = true; # TOML
          programs.shfmt.enable = true;
        };
      };

      flake.hydraJobs = let
        allJobs = {
          blockfrost-platform = lib.genAttrs (config.systems ++ ["x86_64-windows"]) (
            targetSystem: inputs.self.internal.${targetSystem}.package
          );
          devshell = lib.genAttrs config.systems (
            targetSystem: inputs.self.devShells.${targetSystem}.default
          );
          archive = lib.genAttrs (config.systems ++ ["x86_64-windows"]) (
            targetSystem: inputs.self.internal.${targetSystem}.archive
          );
          installer = {
            x86_64-windows = inputs.self.internal.x86_64-windows.installer;
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

      flake.nixConfig = {
        extra-substituters = ["https://cache.iog.io"];
        extra-trusted-public-keys = ["hydra.iohk.io:f/Ea+s+dFdN+3Y/G+FDgSq+a5NEWhJGzdjvKNGv0/EQ="];
        allow-import-from-derivation = "true";
      };
    });
}
