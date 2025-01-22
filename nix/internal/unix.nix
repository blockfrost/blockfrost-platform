{
  inputs,
  targetSystem,
}:
# For now, let's keep all UNIX definitions together, until they diverge more in the future.
assert __elem targetSystem ["x86_64-linux" "aarch64-linux" "aarch64-darwin" "x86_64-darwin"]; let
  buildSystem = targetSystem;
  pkgs = inputs.nixpkgs.legacyPackages.${buildSystem};
  inherit (pkgs) lib;
in rec {
  craneLib = inputs.crane.mkLib pkgs;

  src = craneLib.cleanCargoSource ../../.;

  commonArgs = {
    inherit src;
    strictDeps = true;
    nativeBuildInputs = lib.optionals pkgs.stdenv.isLinux [
      pkgs.pkg-config
      #testgen-hs
    ];
    TESTGEN_HS_PATH = lib.getExe testgen-hs; # Don’t try to download it in `build.rs`.
    buildInputs =
      lib.optionals pkgs.stdenv.isLinux [
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

  package = craneLib.buildPackage (commonArgs
    // {
      inherit cargoArtifacts;
      doCheck = false; # we run tests with `cargo-nextest` below
      postInstall = ''
        chmod -R +w $out
        mv $out/bin $out/libexec
        ln -sf ${testgen-hs}/bin $out/libexec/testgen-hs
        mkdir -p $out/bin
        ln -sf $out/libexec/blockfrost-platform $out/bin/
      '';
    });

  cargoChecks = {
    cargo-clippy = craneLib.cargoClippy (commonArgs
      // {
        inherit cargoArtifacts;
        # Maybe also add `--deny clippy::pedantic`?
        cargoClippyExtraArgs = "--all-targets --all-features -- --deny warnings";
      });

    cargo-doc = craneLib.cargoDoc (commonArgs
      // {
        RUSTDOCFLAGS = "-D warnings";
        inherit cargoArtifacts;
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
        inherit cargoArtifacts;
        cargoNextestExtraArgs = "--lib";
      });
  };

  cardano-node-flake = let
    unpatched = inputs.cardano-node;
  in
    (import inputs.flake-compat {
      src =
        if targetSystem != "aarch64-darwin"
        then unpatched
        else {
          outPath = toString (pkgs.runCommand "source" {} ''
            cp -r ${unpatched} $out
            chmod -R +w $out
            cd $out
            echo ${lib.escapeShellArg (builtins.toJSON [targetSystem])} $out/nix/supported-systems.nix
          '');
          inherit (unpatched) rev shortRev lastModified lastModifiedDate;
        };
    })
    .defaultNix;

  cardano-node-packages =
    {
      x86_64-linux = cardano-node-flake.hydraJobs.x86_64-linux.musl;
      x86_64-darwin = cardano-node-flake.packages.x86_64-darwin;
      aarch64-darwin = cardano-node-flake.packages.aarch64-darwin;
    }
    .${targetSystem};

  inherit (cardano-node-packages) cardano-node cardano-cli;

  cardano-node-configs = builtins.path {
    name = "cardano-playground-configs";
    path = inputs.cardano-playground + "/static/book.play.dev.cardano.org/environments";
  };

  testgen-hs-flake = (import inputs.flake-compat {src = inputs.testgen-hs;}).defaultNix;

  testgen-hs = testgen-hs-flake.packages.${targetSystem}.default;

  stateDir =
    if pkgs.stdenv.isDarwin
    then "Library/Application Support/blockfrost-platform"
    else ".local/share/blockfrost-platform";

  runNode = network:
    pkgs.writeShellScriptBin "run-node-${network}" ''
      stateDir="$HOME"/${lib.escapeShellArg (stateDir + "/" + network)}
      mkdir -p "$stateDir"
      set -x
      exec ${lib.getExe cardano-node} run \
        --config ${cardano-node-configs}/${network}/config.json \
        --topology ${cardano-node-configs}/${network}/topology.json \
        --socket-path "$stateDir"/node.socket \
        --database-path "$stateDir"/chain
    ''
    // {meta.description = "Runs cardano-node on ${network}";};

  # For generating a signing key from a recovery phrase. It’s a little
  # controversial to download a binary, but we only need it for the devshell. If
  # needed, we can use the source instead.
  cardano-address = let
    release = "v2024-09-29";
    baseUrl = "https://github.com/cardano-foundation/cardano-wallet/releases/download/${release}/cardano-wallet";
    archive = pkgs.fetchzip {
      name = "cardano-wallet-${release}";
      url =
        {
          "x86_64-linux" = "${baseUrl}-${release}-linux64.tar.gz";
          "x86_64-darwin" = "${baseUrl}-${release}-macos-intel.tar.gz";
          "aarch64-darwin" = "${baseUrl}-${release}-macos-silicon.tar.gz";
        }
        .${targetSystem};
      hash =
        {
          "x86_64-linux" = "sha256-EOe6ooqvSGylJMJnWbqDrUIVYzwTCw5Up/vU/gPK6tE=";
          "x86_64-darwin" = "sha256-POUj3Loo8o7lBI4CniaA/Z9mTRAmWv9VWAdtcIMe27I=";
          "aarch64-darwin" = "sha256-+6bzdUXnJ+nnYdZuhLueT0+bYmXzwDXTe9JqWrWnfe4=";
        }
        .${targetSystem};
    };
  in
    pkgs.runCommandNoCC "cardano-address" {
      meta.description = "Command-line for address and key manipulation in Cardano";
    } ''
      mkdir -p $out/bin $out/libexec
      cp ${archive}/cardano-address $out/libexec/
      ${lib.optionalString pkgs.stdenv.isDarwin ''
        cp ${archive}/{libz,libiconv.2,libgmp.10,libffi.8}.dylib $out/libexec
      ''}
      ln -sf $out/libexec/cardano-address $out/bin/
    '';

  tx-build = let
    onPath = with pkgs; [
      bash
      coreutils
      gnused
      gnugrep
      jq
      bc
      cardano-cli
      cardano-address
    ];
  in
    pkgs.writeShellScriptBin "tx-build" ''
      set -euo pipefail
      export PATH=${lib.escapeShellArg (lib.makeBinPath onPath)}:"$PATH"
      if [ -z "''${CARDANO_NODE_SOCKET_PATH:-}" ] ; then
        if [[ "''${1:-}" =~ ^(preview|preprod|mainnet)$ ]]; then
          export CARDANO_NODE_SOCKET_PATH="$HOME"/${lib.escapeShellArg stateDir}/"$1"/node.socket
        fi
      fi
      exec ${./tx-build.sh} "$@"
    ''
    // {meta.description = "Builds a CBOR transaction for testing ‘/tx/submit’";};
}
