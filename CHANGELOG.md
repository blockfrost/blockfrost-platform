## [1.0.0-rc.1] - 2026-04-15

### Added

- Hydra micropayments between gateway and platform, and between gateway and end users
- `blockfrost-gateway` is now part of the monorepo and shipped together with the platform
- CIP-129 support
- Multi-arch OCI (Docker) images
- Data node monitoring with version and revision reporting
- Concurrency limiting

### Changed

- Updated `cardano-node` to 10.6.3
- Updated Dolos to 1.0.3
- Dolos config is now a generic data node config
- Unified error handling and logging
- Improved data node error logging
- Only single data node support

### Fixed

- Tests: hardcoded `project_id` now correctly resolved from environment
- Gateway no longer logs secrets
- Hydra: reset stale credits on Head `Close` and seed Gateway balance on `Open`
- Hydra: `Commit` each participant as soon as their own node hits `Initial`
- Addresses dummy response
- Incorrect `drep_id` mapping removed
- Genesis preview start
- `glibc` version alignment in the Dockerfile
- Windows build of `blockfrost-gateway.exe` now includes `libpq`
- Installers fixed and verified before release

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
