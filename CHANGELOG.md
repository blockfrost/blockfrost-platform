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
