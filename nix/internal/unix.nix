{
  inputs,
  targetSystem,
}:
assert __elem targetSystem ["x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin"]; let
  buildSystem = targetSystem;
  pkgs = inputs.nixpkgs.legacyPackages.${buildSystem};
  inherit (pkgs) lib;
  extendForTarget = unix:
    (
      if pkgs.stdenv.isLinux
      then import ./linux.nix
      else if pkgs.stdenv.isDarwin
      then import ./darwin.nix
      else throw "can’t happen"
    ) {inherit inputs targetSystem unix;};
in
  extendForTarget rec {
    craneLib = inputs.crane.mkLib pkgs;

    src = lib.cleanSourceWith {
      src = lib.cleanSource ../../.;
      filter = path: type:
        craneLib.filterCargoSources path type
        || lib.hasSuffix ".sql" path
        || lib.hasSuffix "/LICENSE" path;
      name = "source";
    };

    commonArgs = {
      inherit src;
      strictDeps = true;
      nativeBuildInputs = lib.optionals pkgs.stdenv.isLinux [
        pkgs.pkg-config
      ];
      buildInputs =
        [pkgs.postgresql]
        ++ lib.optionals pkgs.stdenv.isLinux [
          pkgs.openssl
        ]
        ++ lib.optionals pkgs.stdenv.isDarwin [
          pkgs.libiconv
          pkgs.darwin.apple_sdk_12_3.frameworks.SystemConfiguration
          pkgs.darwin.apple_sdk_12_3.frameworks.Security
          pkgs.darwin.apple_sdk_12_3.frameworks.CoreFoundation
        ];
    };

    # For better caching:
    cargoArtifacts = craneLib.buildDepsOnly commonArgs;

    packageName = (craneLib.crateNameFromCargoToml {cargoToml = src + "/Cargo.toml";}).pname;

    GIT_REVISION = inputs.self.rev or "dirty";

    package = craneLib.buildPackage (commonArgs
      // {
        inherit cargoArtifacts GIT_REVISION;
        doCheck = false; # we run tests with `cargo-nextest` below
        meta.mainProgram = packageName;
      });

    # We use a newer `rustfmt`:
    rustfmt = inputs.fenix.packages.${pkgs.system}.stable.rustfmt;

    cargoChecks = {
      cargo-clippy = craneLib.cargoClippy (commonArgs
        // {
          inherit cargoArtifacts GIT_REVISION;
          # Maybe also add `--deny clippy::pedantic`?
          cargoClippyExtraArgs = "--all-targets --all-features -- --deny warnings";
        });

      cargo-doc = craneLib.cargoDoc (commonArgs
        // {
          inherit cargoArtifacts GIT_REVISION;
          RUSTDOCFLAGS = "-D warnings";
        });

      cargo-audit = craneLib.cargoAudit {
        inherit src;
        inherit (inputs) advisory-db;
      };

      cargo-deny = craneLib.cargoDeny {
        inherit src;
      };

      cargo-test = craneLib.cargoNextest (commonArgs
        // {
          inherit cargoArtifacts GIT_REVISION;
        });
    };
  }
