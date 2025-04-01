{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";
    crane.url = "github:ipetkov/crane";
    fenix.url = "github:nix-community/fenix";
    fenix.inputs.nixpkgs.follows = "nixpkgs";
    devshell.url = "github:numtide/devshell";
    devshell.inputs.nixpkgs.follows = "nixpkgs";
    advisory-db.url = "github:rustsec/advisory-db";
    advisory-db.flake = false;
  };

  outputs = inputs: let
    inherit (inputs.nixpkgs) lib;
  in
    inputs.flake-parts.lib.mkFlake {inherit inputs;} ({config, ...}: {
      imports = [
        inputs.devshell.flakeModule
        inputs.treefmt-nix.flakeModule
      ];

      flake.internal = lib.genAttrs config.systems (
        targetSystem: import ./nix/internal/unix.nix {inherit inputs targetSystem;}
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
        packages.default = internal.package;

        devshells.default = import ./nix/devshells.nix {inherit inputs;};

        checks = internal.cargoChecks // internal.nixChecks;

        treefmt = {pkgs, ...}: {
          projectRootFile = "flake.nix";
          programs.alejandra.enable = true; # Nix
          programs.prettier.enable = true;
          programs.rustfmt.enable = true;
          programs.rustfmt.package = internal.rustfmt;
          settings.formatter.rustfmt.options = [
            "--config-path"
            (builtins.path {
              name = "rustfmt.toml";
              path = ./rustfmt.toml;
            })
          ];
          programs.yamlfmt.enable = pkgs.system != "x86_64-darwin"; # a treefmt-nix+yamlfmt bug on Intel Macs
          programs.taplo.enable = true; # TOML
          programs.shfmt.enable = true;
        };
      };
    });
}
