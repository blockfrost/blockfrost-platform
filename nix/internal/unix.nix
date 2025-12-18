{
  inputs,
  targetSystem,
}:
assert builtins.elem targetSystem ["x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin"]; let
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
    rustPackages = inputs.fenix.packages.${pkgs.system}.stable;

    craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustPackages.toolchain;

    src = lib.cleanSourceWith {
      src = lib.cleanSource ../../.;
      filter = path: type:
        craneLib.filterCargoSources path type
        || lib.hasSuffix ".sql" path
        || lib.hasSuffix "/LICENSE" path;
      name = "source";
    };

    commonArgs =
      {
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
      }
      // lib.optionalAttrs pkgs.stdenv.isDarwin {
        # for bindgen, used by libproc, used by metrics_process
        LIBCLANG_PATH = "${lib.getLib pkgs.llvmPackages.libclang}/lib";
      }
      // lib.optionalAttrs pkgs.stdenv.isLinux {
        # The linker bundled with Fenix has wrong interpreter path, and it fails with ENOENT, so:
        RUSTFLAGS = "-Clink-arg=-fuse-ld=bfd";
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
        postInstall = ''
          mv $out/bin $out/libexec
          mkdir -p $out/bin
          ( cd $out/bin && ln -s ../libexec/${packageName} ./ ; )
          ln -s ${hydra-node}/bin/hydra-node $out/libexec/
        '';
      }
      // (builtins.listToAttrs hydraScriptsEnvVars));

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

    nixChecks = {
      nix-statix =
        pkgs.runCommandNoCC "nix-statix" {
          buildInputs = [pkgs.statix];
        } ''
          touch $out
          cd ${inputs.self}
          exec statix check .
        '';

      nix-deadnix =
        pkgs.runCommandNoCC "nix-deadnix" {
          buildInputs = [pkgs.deadnix];
        } ''
          touch $out
          cd ${inputs.self}
          exec deadnix --fail .
        '';

      nix-nil =
        pkgs.runCommandNoCC "nix-nil" {
          buildInputs = [pkgs.nil];
        } ''
          ec=0
          touch $out
          cd ${inputs.self}
          find . -type f -iname '*.nix' | while IFS= read -r file; do
            nil diagnostics "$file" || ec=1
          done
          exit $ec
        '';

      # From `nixd`:
      nix-nixf =
        pkgs.runCommandNoCC "nix-nil" {
          buildInputs = [pkgs.nixf pkgs.jq];
        } ''
          ec=0
          touch $out
          cd ${inputs.self}
          find . -type f -iname '*.nix' | while IFS= read -r file; do
            errors=$(nixf-tidy --variable-lookup --pretty-print <"$file" | jq -c '.[]' | sed -r "s#^#$file: #")
            if [ -n "$errors" ] ; then
              cat <<<"$errors"
              echo
              ec=1
            fi
          done
          exit $ec
        '';
    };

    hydra-flake = (import inputs.flake-compat {src = inputs.hydra;}).defaultNix;

    hydraVersion = hydra-flake.legacyPackages.${targetSystem}.hydra-node.identifier.version;

    hydraNetworksJson = builtins.path {
      path = hydra-flake + "/hydra-node/networks.json";
    };

    hydraScriptsEnvVars = map (network: {
      name = "HYDRA_SCRIPTS_TX_ID_${lib.strings.toUpper network}";
      value = (builtins.fromJSON (builtins.readFile hydraNetworksJson)).${network}.${hydraVersion};
    }) ["mainnet" "preprod" "preview"];

    hydra-node = lib.recursiveUpdate hydra-flake.packages.${targetSystem}.hydra-node {
      meta.description = "Layer 2 scalability solution for Cardano";
    };

    cardano-node-flake = let
      unpatched = inputs.cardano-node;
    in
      (import inputs.flake-compat {
        src =
          if targetSystem != "aarch64-darwin" && targetSystem != "aarch64-linux"
          then unpatched
          else {
            outPath = toString (pkgs.runCommand "source" {} ''
              cp -r ${unpatched} $out
              chmod -R +w $out
              cd $out
              echo ${lib.escapeShellArg (builtins.toJSON [targetSystem])} >$out/nix/supported-systems.nix
              ${lib.optionalString (targetSystem == "aarch64-linux") ''
                sed -r 's/"-fexternal-interpreter"//g' -i $out/nix/haskell.nix
              ''}
            '');
            inherit (unpatched) rev shortRev lastModified lastModifiedDate;
          };
      })
      .defaultNix;

    cardano-node-packages =
      {
        x86_64-linux = cardano-node-flake.hydraJobs.x86_64-linux.musl;
        inherit (cardano-node-flake.packages) x86_64-darwin aarch64-darwin aarch64-linux;
      }
      .${
        targetSystem
      };

    inherit (cardano-node-packages) cardano-cli;
  }
