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

    src = craneLib.cleanCargoSource ../../.;

    commonArgs =
      {
        inherit src;
        strictDeps = true;
        nativeBuildInputs = lib.optionals pkgs.stdenv.isLinux [
          pkgs.pkg-config
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
      }
      // lib.optionalAttrs pkgs.stdenv.isDarwin {
        # for bindgen, used by libproc, used by metrics_process
        LIBCLANG_PATH = "${lib.getLib pkgs.llvmPackages.libclang}/lib";
      };

    # For better caching:
    cargoArtifacts = craneLib.buildDepsOnly commonArgs;

    packageName = (craneLib.crateNameFromCargoToml {cargoToml = builtins.path {path = src + "/Cargo.toml";};}).pname;

    cargoToml = builtins.fromTOML (builtins.readFile (builtins.path {path = src + "/Cargo.toml";}));

    GIT_REVISION = inputs.self.rev or "dirty";

    package = craneLib.buildPackage (commonArgs
      // {
        inherit cargoArtifacts GIT_REVISION;
        doCheck = false; # we run tests with `cargo-nextest` below
        postInstall = ''
          chmod -R +w $out
          mv $out/bin $out/libexec
          mkdir -p $out/bin
          ln -sf $out/libexec/${packageName} $out/bin/
        '';
        meta = {
          mainProgram = packageName;
          license =
            if cargoToml.package.license == "Apache-2.0"
            then lib.licenses.asl20
            else throw "unknown license in Cargo.toml: ${cargoToml.package.license}";
          inherit (cargoToml.package) description homepage;
        };
      });

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
          cargoNextestExtraArgs = "--lib";
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
      path = inputs.cardano-playground + "/static/book.play.dev.cardano.org/environments";
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
      then "Library/Application Support/${packageName}"
      else ".local/share/${packageName}";

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

    releaseBaseUrl = "https://github.com/blockfrost/blockfrost-platform/releases/download/${package.version}";

    # This works for both Linux and Darwin, but we mostly use it on Linux:
    curl-bash-install =
      pkgs.runCommandNoCC "curl-bash-install" {
        nativeBuildInputs = with pkgs; [shellcheck];
        projectName = packageName;
        projectVersion = package.version;
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
        meta = {
          mainProgram = "dolos";
          description = "Cardano Data Node";
        };
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

    blockfrost-tests = make-blockfrost-tests "preview";

    make-blockfrost-tests = network: let
      inherit (pkgs) nodePackages;
    in
      pkgs.writeShellApplication {
        name = "blockfrost-tests";
        runtimeInputs = with pkgs; [
          bash
          coreutils
          nodePackages.nodejs
          nodePackages.yarn
          (python3.withPackages (ps: with ps; [portpicker]))
          wait4x
        ];
        text = ''
          set -euo pipefail

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
          for v in PROJECT_ID SUBMIT_MNEMONIC ; do
            require_env "$v"
          done
          if (( missing )); then
            exit 1
          fi

          export NETWORK=${lib.escapeShellArg network}

          platform_port=$(python3 -m portpicker)

          ${lib.getExe package} \
            --server-address 127.0.0.1 \
            --server-port "$platform_port" \
            --log-level info \
            --node-socket-path "''${CARDANO_NODE_SOCKET_PATH:-/run/cardano-node/node.socket}" \
            --mode compact \
            --solitary \
            --dolos-endpoint "''${DOLOS_ENDPOINT:-http://127.0.0.1:3010}" \
            --dolos-timeout-sec 30 \
            &
          platform_pid=$!

          export SERVER_URL="http://127.0.0.1:$platform_port"

          sleep 1
          wait4x http "$SERVER_URL" --expect-status-code 200 --timeout 60s --interval 1s

          cp -r ${inputs.blockfrost-tests}/. "$tmpdir"/.
          chmod -R u+w,g+w "$tmpdir"
          cd "$tmpdir"
          cat ${../../tests/data/supported_endpoints.json} >endpoints-allowlist.json

          set -x
          node --version
          yarn --version

          yarn install
          yarn test:preview
        '';
      };
  }
