{
  inputs = {
    nixpkgs.url = "github:nixos/nixpkgs/nixpkgs-unstable";
    flake-parts.url = "github:hercules-ci/flake-parts";
    treefmt-nix.url = "github:numtide/treefmt-nix";
    treefmt-nix.inputs.nixpkgs.follows = "nixpkgs";
    crane.url = "github:ipetkov/crane";
    flake-compat.url = "github:input-output-hk/flake-compat";
    flake-compat.flake = false;
    cardano-node.url = "github:IntersectMBO/cardano-node/10.1.2";
    cardano-node.flake = false; # otherwise, +2k dependencies we don’t really use
    testgen-hs.url = "github:input-output-hk/testgen-hs/10.1.2.1"; # make sure it follows cardano-node
    testgen-hs.flake = false; # otherwise, +2k dependencies we don’t really use
    devshell.url = "github:numtide/devshell";
    devshell.inputs.nixpkgs.follows = "nixpkgs";
    cardano-playground.url = "github:input-output-hk/cardano-playground/b4f47fd78beec0ea1ed880d6f0b794919e0c0463";
    cardano-playground.flake = false; # otherwise, +9k dependencies in flake.lock…
  };

  outputs = inputs:
    inputs.flake-parts.lib.mkFlake {inherit inputs;} ({config, ...}: {
      imports = [
        inputs.devshell.flakeModule
        inputs.treefmt-nix.flakeModule
      ];

      flake.internal =
        inputs.nixpkgs.lib.genAttrs config.systems (
          targetSystem: import ./nix/internal/unix.nix {inherit inputs targetSystem;}
        )
        // inputs.nixpkgs.lib.genAttrs ["x86_64-windows"] (
          targetSystem: import ./nix/internal/windows.nix {inherit inputs targetSystem;}
        );

      systems = ["x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin"];
      perSystem = {
        config,
        system,
        pkgs,
        ...
      }: {
        packages = let
          internal = inputs.self.internal.${system};
        in
          {
            default = internal.package;
            inherit (internal) tx-build cardano-address testgen-hs;
          }
          // (inputs.nixpkgs.lib.optionalAttrs (system == "x86_64-linux") {
            default-x86_64-windows = inputs.self.internal.x86_64-windows.package;
          });

        devshells.default = import ./nix/devshells.nix {inherit inputs;};

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
          programs.rustfmt.enable = true;
          programs.yamlfmt.enable = true;
          programs.taplo.enable = true; # TOML
          programs.shfmt.enable = true;
        };
      };
    });
}
