## Unreleased

### Changed

- In CLI `no_metrics` renamed to `metrics`

### Removed

- `network` parameter from CLI. It's resolved automatically now.

### Added

- Set custom genesis config
- Load balancing over a WebSocket (eliminating the need for public IP in the future)
- Expose a `health_errors_total` gauge in metrics

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
