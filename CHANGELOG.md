## [1.0.0-rc.1] - 2026-04-xx

### Highlights

This release introduces two major integrations:

<img src="docs/hydra.png" alt="" height="30" align="center" /> &nbsp; + &nbsp;<img src="docs/dolos.png" alt="" height="40" align="center" />

### Added

- [Hydra](https://hydra.family/head-protocol/) integration: Blockfrost API access to Hydra Heads
- [Dolos](https://github.com/txpipe/dolos) integration as a data node (alternative to `cardano-db-sync`)
- Hydra micropayments between Gateway and Platform, and between Gateway and end users
- Hydra Head with `--blockfrost` integration test
- `hydra-node` 1.0.0 bundled with the installers (no external `cardano-cli` required at runtime)
- `blockfrost-sdk-bridge` binary shipped via installers, Nix packages, and release artifacts
- `blockfrost-gateway` is now part of the monorepo and shipped together with the platform
- CIP-129 support
- Multi-arch OCI (Docker) images
- Data node monitoring with version and revision reporting
- Concurrency limiting
- `BLOCKFROST_GATEWAY_URL` environment variable to override the Gateway URL
- WebSocket end-to-end integration tests
- Weekly `flake.lock` update workflow
- CI checks for unused Cargo dependencies and a runnability check for the published OCI image

### Changed

- Updated `cardano-node` to 10.6.3
- Dolos config is now a generic data node config
- Unified error handling and logging
- Improved data node error logging
- Workspace reorganization: `src` → `crates/platform`, node code extracted to its own crate, integration tests moved to a separate crate
- Unified and aligned all Cargo workspace dependencies
- Rust toolchain bumped to 1.93.1, edition updated
- CI: 3rd-party Actions pinned to immutable commits (supply-chain hardening)
- CI: incremental builds disabled for faster builds
- Devshell: added `midnight-node` and `midnight-indexer`
- Windows NSIS installer is now HiDPI-aware; dropped `inputs.nixpkgs-nsis`
- `latest` OCI tag is no longer pushed for pre-releases
- Docker: `libssl-dev` unpinned; `glibc` version aligned
- Node configs now pulled from the Operations book
- Docs: migrated from yarn to pnpm, ESLint bumped to v9

### Fixed

- Tests: hardcoded `project_id` now correctly resolved from environment
- Gateway no longer logs secrets
- Hydra: reset stale credits on Head `Close` and seed Gateway balance on `Open`
- Hydra: `Commit` each participant as soon as their own node hits `Initial`
- Hydra: retry when returning funds to `SUBMIT_MNEMONIC`
- Hydra: test resiliency against concurrent tx submission races and `OutsideValidityIntervalUTxO`
- `sdk-bridge`: correct naming of protocol fields (`platform` → `bridge`)
- `sdk-bridge`: reconnect with exponential backoff when the WebSocket drops
- `sdk-bridge`: periodically clean up timed-out in-flight requests
- Immediately run built binaries; fail early if no `hydra-node`
- Addresses dummy response; improved asset and block validation
- Incorrect `drep_id` mapping removed
- Epoch number parameter handling
- Genesis preview start
- WebSocket now correctly forwards the request query string
- Gateway: correct network detection for Preprod
- `cardano-cli` removed from bundles; `CARDANO_NODE_NETWORK_ID` set in Rust
- Windows build of `blockfrost-gateway.exe` (multiple fixes around `libpq`, `reqwest`, cross-build)
- `glibc` version alignment in the Dockerfile
- Windows build of `blockfrost-gateway.exe` now includes `libpq`
- Installers fixed and verified before release
- Prevent `unknown/unknown` architecture label in the GitHub UI for OCI images

## [0.0.3-rc.3] - 2025-09-23

### Removed

- `network` parameter from CLI. It's resolved automatically now.

### Added

- Set custom genesis config
- Load balancing over a WebSocket (eliminating the need for public IP in the future)
- Expose a `health_errors_total` gauge in metrics
- More comprehensive error reporting under `GET /`
- NixOS service (module)
- Run original `blockfrost-tests` against the Platform

#### New endpoints from Dolos

**General**

- `/network`
- `/network/eras`
- `/genesis`

**Transactions**

- `/txs/{hash}/cbor`
- `/txs/{hash}/utxos`
- `/txs/{hash}/metadata`
- `/txs/{hash}/metadata/cbor`
- `/txs/{hash}/withdrawals`
- `/txs/{hash}/delegations`
- `/txs/{hash}/redeemers`
- `/txs/{hash}/mirs`
- `/txs/{hash}/pool_retires`
- `/txs/{hash}/pool_updates`
- `/txs/{hash}/stakes`

**Blocks**

- `/blocks/latest`
- `/blocks/latest/txs`
- `/blocks/{hash_or_number}`
- `/blocks/{hash_or_number}/next`
- `/blocks/{hash_or_number}/previous`
- `/blocks/{hash_or_number}/txs`
- `/blocks/slot/{slot}`

**Addresses**

- `/addresses/{address}/utxos`
- `/addresses/{address}/transactions`

**Accounts**

- `/accounts/{stake_address}`
- `/accounts/{stake_address}/rewards`
- `/accounts/{stake_address}/addresses`
- `/accounts/{stake_address}/delegations`
- `/accounts/{stake_address}/registrations`

**Assets**

- `/assets/{asset}`

**Governance**

- `/governance/dreps/{drep_id}`

**Metadata**

- `/metadata/txs/labels/{label}`
- `/metadata/txs/labels/{label}/cbor`

**Pools**

- `/pools/extended`
- `/pools/{pool_id}/delegators`

**Epochs**

- `/epochs/{number}/parameters`
- `/epochs/latest/parameters`

### Fixed

- Trailing slash in `GET /{uuid}/` works again
- Health reporting while still syncing in the Byron era
- Native (not cross-compiled) `aarch64-linux` builds
- Docs improvements
- TLS support for the WebSocket connection with the Gateway

## [0.0.2] - 2025-03-20

### Changed

- Enable metrics endpoint by default

### Added

- Expose process metrics (memory, CPU time, fds, threads)
- Add more logs and finer details to `NodeClient::submit_transaction`
- Header `blockfrost-platform-response` in `tx_submit` endpoint
- Add `aarch64-linux` builds to release artifacts and installers.

### Fixed

- Node connections are now invalidated on unexpected transaction submission errors.
- Node connection metrics inconsistency caused by an initialization timing issue.
- Configure local IP address to bind to with `std::net` types.

## [0.0.1] - 2025-02-13

### Added

- Initial release
