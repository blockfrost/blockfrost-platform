{
  inputs,
  targetSystem,
}:
# For now, let's keep all UNIX definitions together, until they diverge more in the future.
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

    packageName = craneLib.crateNameFromCargoToml {cargoToml = builtins.path {path = src + "/crates/platform/Cargo.toml";};};

    commonArgs =
      {
        inherit src;
        inherit (packageName) pname;
        strictDeps = true;
        nativeBuildInputs = lib.optionals pkgs.stdenv.isLinux [
          pkgs.pkg-config
        ];
        TESTGEN_HS_PATH = lib.getExe testgen-hs; # Don’t try to download it in `build.rs`.
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

    workspaceCargoToml = builtins.fromTOML (builtins.readFile (builtins.path {path = src + "/Cargo.toml";}));
    platformCargoToml = builtins.fromTOML (builtins.readFile (builtins.path {path = src + "/crates/platform/Cargo.toml";}));
    gatewayCargoToml = builtins.fromTOML (builtins.readFile (builtins.path {path = src + "/crates/gateway/Cargo.toml";}));

    GIT_REVISION = inputs.self.rev or "dirty";

    blockfrost-platform = craneLib.buildPackage (commonArgs
      // {
        inherit cargoArtifacts GIT_REVISION;
        doCheck = false; # we run tests with `cargo-nextest` below
        postInstall = ''
          chmod -R +w $out
          mv $out/bin $out/libexec
          mkdir -p $out/bin
          ln -sf $out/libexec/${packageName.pname} $out/bin/
        '';
        meta = {
          mainProgram = packageName.pname;
          license =
            if workspaceCargoToml.workspace.package.license == "Apache-2.0"
            then lib.licenses.asl20
            else throw "unknown license in Cargo.toml: ${workspaceCargoToml.workspace.package.license}";
          inherit (platformCargoToml.package) description homepage;
        };
      });

    blockfrost-gateway = craneLib.buildPackage (commonArgs
      // {
        inherit cargoArtifacts GIT_REVISION;
        pname = gatewayCargoToml.package.name;
        doCheck = false; # we run tests with `cargo-nextest` below
        meta.mainProgram = gatewayCargoToml.package.name;
        postInstall = ''
          mv $out/bin $out/libexec
          mkdir -p $out/bin
          ( cd $out/bin && ln -s ../libexec/${gatewayCargoToml.package.name} ./ ; )
          ln -s ${hydra-node}/bin/hydra-node $out/libexec/
        '';
        cargoExtraArgs = "--package blockfrost-gateway";
      }
      // (builtins.listToAttrs hydraScriptsEnvVars));

    cargoChecks = {
      cargo-clippy = craneLib.cargoClippy (commonArgs
        // {
          inherit cargoArtifacts GIT_REVISION;
          # Maybe also add `--deny clippy::pedantic`?
          cargoClippyExtraArgs = "--all-targets -- --deny warnings";
        });

      cargo-doc = craneLib.cargoDoc (commonArgs
        // {
          inherit cargoArtifacts GIT_REVISION;
          RUSTDOCFLAGS = "-D warnings";
        });

      cargo-audit = craneLib.cargoAudit {
        inherit (packageName) pname;
        inherit src;
        inherit (inputs) advisory-db;
      };

      cargo-deny = craneLib.cargoDeny {
        inherit (packageName) pname;
        inherit src;
      };

      cargo-test = craneLib.cargoNextest (commonArgs
        // {
          inherit cargoArtifacts GIT_REVISION;
          cargoNextestExtraArgs = "--workspace --lib";
        }
        // (builtins.listToAttrs hydraScriptsEnvVars));
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

    inherit (cardano-node-packages) cardano-node cardano-cli cardano-submit-api;

    cardano-node-configs-verbose = builtins.path {
      name = "cardano-playground-configs";
      path = inputs.cardano-playground + "/docs/environments";
    };

    cardano-node-configs =
      pkgs.runCommandNoCC "cardano-node-configs" {
        buildInputs = with pkgs; [jq];
      } ''
        cp -r ${cardano-node-configs-verbose} $out
        chmod -R +w $out
        find $out -name 'config.json' | while IFS= read -r configFile ; do
          jq '.
            | .TraceConnectionManager = false
            | .TracePeerSelection = false
            | .TracePeerSelectionActions = false
            | .TracePeerSelectionCounters = false
            | .TraceInboundGovernor = false
          ' "$configFile" >tmp.json
          mv tmp.json "$configFile"
        done
      '';

    generated-dir = pkgs.runCommandNoCC "generated-dir" {} ''
      mkdir -p $out
      ln -s ${cardano-node-configs} $out/cardano-node-configs
      ln -s ${dolos-configs} $out/dolos-configs
    '';

    testgen-hs-flake = (import inputs.flake-compat {src = inputs.testgen-hs;}).defaultNix;

    testgen-hs = testgen-hs-flake.packages.${targetSystem}.default;

    stateDir =
      if pkgs.stdenv.isDarwin
      then "Library/Application Support/${packageName.pname}"
      else ".local/share/${packageName.pname}";

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
    cardano-address =
      if targetSystem == "aarch64-linux"
      then
        pkgs.writeShellApplication {
          name = "cardano-address";
          text = ''
            echo >&2 "TODO: unimplemented: compile \`cardano-address\` for \`${targetSystem}\`!"
            exit 1
          '';
        }
      else let
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
            .${
              targetSystem
            };
          hash =
            {
              "x86_64-linux" = "sha256-EOe6ooqvSGylJMJnWbqDrUIVYzwTCw5Up/vU/gPK6tE=";
              "x86_64-darwin" = "sha256-POUj3Loo8o7lBI4CniaA/Z9mTRAmWv9VWAdtcIMe27I=";
              "aarch64-darwin" = "sha256-+6bzdUXnJ+nnYdZuhLueT0+bYmXzwDXTe9JqWrWnfe4=";
            }
            .${
              targetSystem
            };
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

    tx-build = pkgs.writeShellApplication {
      name = "tx-build";
      runtimeInputs = with pkgs; [
        bash
        coreutils
        gnused
        gnugrep
        jq
        bc
        cardano-cli
        cardano-address
      ];
      text = ''
        set -euo pipefail
        if [ -z "''${CARDANO_NODE_SOCKET_PATH:-}" ] ; then
          if [[ "''${1:-}" =~ ^(preview|preprod|mainnet)$ ]]; then
            export CARDANO_NODE_SOCKET_PATH="$HOME"/${lib.escapeShellArg stateDir}/"$1"/node.socket
          fi
        fi
        ${builtins.readFile ./tx-build.sh}
      '';
      meta.description = "Builds a valid CBOR transaction for testing ‘/tx/submit’";
    };

    releaseBaseUrl = "https://github.com/blockfrost/blockfrost-platform/releases/download/${blockfrost-platform.version}";

    # This works for both Linux and Darwin, but we mostly use it on Linux:
    curl-bash-install =
      pkgs.runCommandNoCC "curl-bash-install" {
        nativeBuildInputs = with pkgs; [shellcheck];
        projectName = packageName.pname;
        projectVersion = blockfrost-platform.version;
        shortRev = inputs.self.shortRev or "dirty";
        baseUrl = releaseBaseUrl;
      } ''
        sha256_x86_64_linux=$(sha256sum ${inputs.self.hydraJobs.archive.x86_64-linux}/*.tar.* | cut -d' ' -f1)
        sha256_aarch64_linux=$(sha256sum ${inputs.self.hydraJobs.archive.aarch64-linux}/*.tar.* | cut -d' ' -f1)
        sha256_x86_64_darwin=$(sha256sum ${inputs.self.hydraJobs.archive.x86_64-darwin}/*.tar.* | cut -d' ' -f1)
        sha256_aarch64_darwin=$(sha256sum ${inputs.self.hydraJobs.archive.aarch64-darwin}/*.tar.* | cut -d' ' -f1)

        export sha256_x86_64_linux
        export sha256_aarch64_linux
        export sha256_x86_64_darwin
        export sha256_aarch64_darwin

        mkdir -p $out
        substituteAll ${./curl-bash-install.sh} $out/curl-bash-install.sh
        chmod +x $out/*.sh
        shellcheck $out/*.sh
      '';

    mithril-client = inputs.mithril.packages.${targetSystem}.mithril-client-cli;

    mithrilGenesisVerificationKeys = {
      preview = builtins.readFile (inputs.mithril + "/mithril-infra/configuration/pre-release-preview/genesis.vkey");
      preprod = builtins.readFile (inputs.mithril + "/mithril-infra/configuration/release-preprod/genesis.vkey");
      mainnet = builtins.readFile (inputs.mithril + "/mithril-infra/configuration/release-mainnet/genesis.vkey");
    };

    mithrilAncillaryVerificationKeys = {
      preview = builtins.readFile (inputs.mithril + "/mithril-infra/configuration/pre-release-preview/ancillary.vkey");
      preprod = builtins.readFile (inputs.mithril + "/mithril-infra/configuration/release-preprod/ancillary.vkey");
      mainnet = builtins.readFile (inputs.mithril + "/mithril-infra/configuration/release-mainnet/ancillary.vkey");
    };

    mithrilAggregator = {
      preview = "https://aggregator.pre-release-preview.api.mithril.network/aggregator";
      preprod = "https://aggregator.release-preprod.api.mithril.network/aggregator";
      mainnet = "https://aggregator.release-mainnet.api.mithril.network/aggregator";
    };

    dolos = craneLib.buildPackage (
      {
        src = inputs.dolos;
        GIT_REVISION = inputs.dolos.rev;
        strictDeps = true;
        nativeBuildInputs =
          [pkgs.gnum4]
          ++ lib.optionals pkgs.stdenv.isLinux [
            pkgs.pkg-config
          ];
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
        doCheck = false; # some unit tests seem to require network access, so they’re failing in the sandbox
        meta = {
          mainProgram = "dolos";
          description = "Cardano Data Node";
        };
      }
      // lib.optionalAttrs pkgs.stdenv.isLinux {
        # The linker bundled with Fenix has wrong interpreter path, and it fails with ENOENT, so:
        RUSTFLAGS = "-Clink-arg=-fuse-ld=bfd";
      }
      // lib.optionalAttrs pkgs.stdenv.isDarwin {
        # for bindgen, used by libproc, used by metrics_process
        LIBCLANG_PATH = "${lib.getLib pkgs.llvmPackages.libclang}/lib";
      }
    );

    dolos-configs = let
      networks = ["mainnet" "preprod" "preview"];
      mkConfig = network: let
        topology = builtins.fromJSON (builtins.readFile "${cardano-node-configs}/${network}/topology.json");
        byronGenesis = builtins.fromJSON (builtins.readFile "${cardano-node-configs}/${network}/byron-genesis.json");
        peerAddr = let first = lib.head topology.bootstrapPeers; in "${first.address}:${toString first.port}";
        magic = toString byronGenesis.protocolConsts.protocolMagic;
      in
        pkgs.writeText "dolos.toml" ''
          [upstream]
          peer_address = "${peerAddr}"
          network_magic = ${magic}
          is_testnet = ${
            if network == "mainnet"
            then "false"
            else "true"
          }

          [storage]
          version = "v1"
          path = "dolos"
          max_wal_history = 25920

          [genesis]
          byron_path = "${cardano-node-configs}/${network}/byron-genesis.json"
          shelley_path = "${cardano-node-configs}/${network}/shelley-genesis.json"
          alonzo_path = "${cardano-node-configs}/${network}/alonzo-genesis.json"
          conway_path = "${cardano-node-configs}/${network}/conway-genesis.json"
          force_protocol = 6

          [sync]
          pull_batch_size = 100

          [submit]

          [serve.grpc]
          listen_address = "[::]:50051"
          permissive_cors = true

          [serve.ouroboros]
          listen_path = "dolos.socket"
          magic = ${magic}

          [serve.minibf]
          listen_address = "[::]:3010"

          [relay]
          listen_address = "[::]:30031"
          magic = ${magic}

          [mithril]
          aggregator = "${mithrilAggregator.${network}}"
          genesis_key = "${mithrilGenesisVerificationKeys.${network}}"

          [logging]
          max_level = "INFO"
          include_tokio = false
          include_pallas = false
          include_grpc = false
        '';
    in
      pkgs.runCommandNoCC "dolos-configs" {} ''
        mkdir -p $out
        ${lib.concatMapStringsSep "\n" (network: ''
            mkdir -p $out/${network}
            cp ${mkConfig network} $out/${network}/dolos.toml
          '')
          networks}
      '';

    runDolos = network:
      pkgs.writeShellScriptBin "run-dolos-${network}" ''
        stateDir="$HOME"/${lib.escapeShellArg (stateDir + "/" + network)}
        mkdir -p "$stateDir"
        cd "$stateDir"
        defaultArgs=(daemon)
        [ "$#" -eq 0 ] && set -- "''${defaultArgs[@]}"
        set -x
        exec ${lib.getExe dolos} \
          --config ${dolos-configs}/${network}/dolos.toml \
          "$@"
      ''
      // {meta.description = "Runs Dolos on ${network}";};

    acropolis-flake = (import inputs.flake-compat {src = inputs.acropolis;}).defaultNix;

    inherit (acropolis-flake.packages.${targetSystem}) acropolis-process-omnibus acropolis-process-replayer;

    blockfrost-tests-preview = make-blockfrost-tests "preview";
    blockfrost-tests-preprod = make-blockfrost-tests "preprod";
    blockfrost-tests-mainnet = make-blockfrost-tests "mainnet";

    make-blockfrost-tests = network: let
      inherit (pkgs) nodePackages;
    in
      pkgs.writeShellApplication {
        name = "blockfrost-tests";
        meta.description = "Runs `blockfrost-tests` on `${network}` against this repository";
        runtimeInputs = with pkgs; [
          bash
          coreutils
          nodePackages.nodejs
          nodePackages.yarn
          curl
          jq
          (python3.withPackages (ps: with ps; [portpicker]))
          wait4x
        ];
        text = ''
          set -euo pipefail

          if [[ -z ''${DOLOS_ENDPOINT+x} ]]; then
            export DOLOS_ENDPOINT="http://127.0.0.1:3010"
            echo >&2 "warning: DOLOS_ENDPOINT is unset; assuming $DOLOS_ENDPOINT"
          fi

          curl -fsSL "''${DOLOS_ENDPOINT}" | jq -r '"Running Dolos " + .version + " (" + .revision + ")"'

          err() { printf "error: %s\n" "$1" >&2; }

          platform_pid=""
          tmpdir="$(mktemp -d)"
          cleanup() {
            cd / && [[ -d "$tmpdir" ]] && rm -rf -- "$tmpdir"
            if [[ -n "$platform_pid" ]] && kill -0 "$platform_pid"; then
              kill -TERM "$platform_pid"
              wait "$platform_pid"
            fi
          }
          trap cleanup EXIT HUP INT TERM

          require_env() {
            local name="$1"
            local val="''${!name-}"
            if [[ -z "$val" ]]; then
              err "$name is not set."
              missing=1
            fi
          }
          missing=0
          for v in PROJECT_ID SUBMIT_MNEMONIC CARDANO_NODE_SOCKET_PATH ; do
            require_env "$v"
          done
          if (( missing )); then
            exit 1
          fi

          export NETWORK=${lib.escapeShellArg network}

          platform_port=$(python3 -m portpicker)

          ${lib.getExe blockfrost-platform} \
            --server-address 127.0.0.1 \
            --server-port "$platform_port" \
            --log-level info \
            --node-socket-path "''${CARDANO_NODE_SOCKET_PATH}" \
            --mode compact \
            --solitary \
            --data-node "''${DOLOS_ENDPOINT}" \
            --data-node-timeout-sec 30 \
            &
          platform_pid=$!

          export SERVER_URL="http://127.0.0.1:$platform_port"

          sleep 1
          wait4x http "$SERVER_URL" --expect-status-code 200 --timeout 60s --interval 1s

          cp -r ${inputs.blockfrost-tests}/. "$tmpdir"/.
          chmod -R u+w,g+w "$tmpdir"
          cd "$tmpdir"
          cat ${../../crates/platform/tests/data/supported_endpoints.json} >endpoints-allowlist.json

          set -x
          node --version
          yarn --version

          yarn install
          yarn test:${lib.escapeShellArg network}
        '';
      };

    # One degree of indirection for the devshell – we don’t want to compile
    # `blockfrost-platform` on each devshell reload.
    run-blockfrost-tests =
      pkgs.writeShellScriptBin "test-blockfrost-tests" ''
        set -euo pipefail
        exec nix run -L $PRJ_ROOT#internal.${pkgs.system}.${blockfrost-tests-preview.name}
      ''
      // {
        meta.description = blockfrost-tests-preview.meta.description;
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

    hydra-test = pkgs.writeShellApplication {
      name = "test-hydra-against-blockfrost";
      meta.description = "Tests a small Hydra cluster, with one member opening head against the Blockfrost API";
      runtimeInputs = with pkgs; [
        bash
        coreutils
        gnused
        gnugrep
        gawk
        jq
        curl
        hydra-node
        cardano-cli
        cardano-address
        (python3.withPackages (ps: with ps; [portpicker]))
        wait4x
        websocat
        etcd
      ];
      runtimeEnv = rec {
        NETWORK = "preview";
        CARDANO_NODE_NETWORK_ID =
          {
            mainnet = "mainnet";
            preprod = 1;
            preview = 2;
          }.${
            NETWORK
          };
        HYDRA_SCRIPTS_TX_ID = (builtins.fromJSON (builtins.readFile hydraNetworksJson)).${NETWORK}.${hydraVersion};
      };
      text = builtins.readFile ./hydra-blockfrost-test.sh;
    };

    midnight = let
      fenix = inputs.fenix.packages.${pkgs.system};

      # A toolchain with the wasm32 target available:
      rustToolchain = fenix.combine [
        fenix.stable.toolchain
        fenix.targets.wasm32-unknown-unknown.stable.rust-std
        fenix.stable.rust-src
        fenix.stable.llvm-tools
      ];

      craneLib = (inputs.crane.mkLib pkgs).overrideToolchain rustToolchain;

      # FIXME: remove after <https://github.com/midnightntwrk/midnight-ledger/pull/81> lands in node/indexer.
      # Move `static/` files in `midnight-ledger` around, since they won’t be available after vendoring:
      fixVendoring'ledger = ps: drv:
        if lib.any (p: lib.hasPrefix "git+https://github.com/midnightntwrk/midnight-ledger" p.source) ps
        then
          drv.overrideAttrs (_old: {
            postPatch = ''
              mkdir -p ledger/static
              mv static/dust ledger/static/
              cp static/version ledger/static/

              mkdir -p transient-crypto/static
              mv static/bls_filecoin_2p14 transient-crypto/static/

              mkdir -p zswap/static
              mv static/zswap zswap/static/
              cp static/version zswap/static/

              find -iname '*.rs' | xargs grep -REl '!\("(\.\./)+static/' | while IFS= read -r file ; do
                sed -re 's,(!\(\")\.\./((\.\./)*static/),\1\2,g' -i "$file"
              done
            '';
          })
        else drv;

      fixDarwin'wasm-opt-sys = p: drv:
        if p.name == "wasm-opt-sys" # && p.version == "0.116.0"
        then
          drv.overrideAttrs (_old: {
            patches = [
              ./midnight--wasm-opt-sys--darwin.patch
            ];
          })
        else drv;

      trulyCommonArgs =
        {
          nativeBuildInputs =
            [
              pkgs.gnum4
              pkgs.protobuf
            ]
            ++ lib.optionals pkgs.stdenv.isLinux [
              pkgs.pkg-config
            ];
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
        }
        // lib.optionalAttrs pkgs.stdenv.isLinux {
          # The linker bundled with Fenix has wrong interpreter path, and it fails with ENOENT, so:
          RUSTFLAGS = "-Clink-arg=-fuse-ld=bfd";
          # The same problem for the Wasm linker:
          CARGO_TARGET_WASM32_UNKNOWN_UNKNOWN_LINKER = "${pkgs.llvmPackages.lld}/bin/wasm-ld";
        }
        // lib.optionalAttrs pkgs.stdenv.isDarwin {
          # for bindgen
          LIBCLANG_PATH = "${lib.getLib pkgs.llvmPackages.libclang}/lib";
        };
    in {
      midnight-node = let
        src = inputs.midnight-node;
        packageName = craneLib.crateNameFromCargoToml {cargoToml = builtins.path {path = src + "/node/Cargo.toml";};};
        baseArgs = {
          inherit (packageName) version pname;
          inherit src;
          strictDeps = true;
        };
        cargoVendorDir = craneLib.vendorCargoDeps (baseArgs
          // {
            overrideVendorGitCheckout = fixVendoring'ledger;
          }
          // lib.optionalAttrs pkgs.stdenv.isDarwin {
            overrideVendorCargoPackage = fixDarwin'wasm-opt-sys;
          });
        commonArgs =
          baseArgs
          // trulyCommonArgs
          // {
            inherit cargoVendorDir;

            # FIXME: `frame-storage-access-test-runtime`’s `build.rs` script fails otherwise, it’d be good to fix, see:
            # <https://github.com/paritytech/polkadot-sdk/blob/6fd693e6d9cfa46cd2acbcb41cd5b0451a62d67c/substrate/utils/frame/storage-access-test-runtime/build.rs>
            SKIP_WASM_BUILD = 1;

            # FIXME: remove after <https://github.com/midnightntwrk/midnight-node/pull/179>.
            postPatch = ''
              mkdir -p docs/src
              touch docs/src/lib.rs
            '';
          };
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;
      in
        craneLib.buildPackage (commonArgs
          // {
            inherit cargoArtifacts;
            doCheck = false; # we run tests elsewhere
          });

      midnight-indexer = let
        src = inputs.midnight-indexer;
        baseArgs = {
          pname = "midnight-indexer";
          inherit src;
          strictDeps = true;
        };
        cargoVendorDir = craneLib.vendorCargoDeps (baseArgs
          // {
            overrideVendorGitCheckout = fixVendoring'ledger;
          }
          // lib.optionalAttrs pkgs.stdenv.isDarwin {
            overrideVendorCargoPackage = fixDarwin'wasm-opt-sys;
          });
        commonArgs =
          baseArgs
          // trulyCommonArgs
          // {
            inherit cargoVendorDir;
          };
        cargoArtifacts = craneLib.buildDepsOnly commonArgs;

        # XXX: Most of their binaries need to be build with the `cloud` feature,
        # or you’ll hit `unimplemented!()`.
        packagesCloud = craneLib.buildPackage (commonArgs
          // {
            inherit cargoArtifacts;
            pname = commonArgs.pname + "-cloud";
            doCheck = false; # we run tests elsewhere
            cargoExtraArgs = "--features cloud";
          });

        # XXX: But `indexer-standalone` needs the `standalone` flag. And you can’t mix the flags.
        packagesStandalone = craneLib.buildPackage (commonArgs
          // {
            inherit cargoArtifacts;
            pname = commonArgs.pname + "-standalone";
            doCheck = false; # we run tests elsewhere
            cargoExtraArgs = "-p indexer-standalone --features standalone";
          });

        # XXX: But you can mix resulting binaries:
        packages = pkgs.stdenv.mkDerivation {
          inherit (packagesCloud) pname version;
          buildCommand = ''
            mkdir -p $out
            cp -vr ${packagesCloud}/bin $out/
            chmod -R +w $out
            cp -vf ${packagesStandalone}/bin/indexer-standalone $out/bin/
          '';
          meta.mainProgram = "indexer-standalone";
        };
      in
        packages;
    };
  }
