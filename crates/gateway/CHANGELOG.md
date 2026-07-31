## Unreleased

### Added

- Per-relay `blockfrost_gateway_relay_healthy`, `blockfrost_gateway_relay_data_node_up`, and `blockfrost_gateway_relay_info` metrics in `GET /metrics` (and the same data points in `GET /stats`)
- `blockfrost_gateway_healthy` metric in `GET /metrics`, mirroring the `healthy` field of `GET /`
- Prometheus metrics endpoint `GET /metrics` exposing per-relay stats (connection status, WebSocket RTT, connected-since timestamp, request/response counters) and PostgreSQL connection-pool gauges (max size, open, available, waiting)
- Prometheus counter `blockfrost_gateway_http_requests_total` with `method`, `route`, and `status_code` labels for Gateway API requests
- `blockfrost_gateway_build_info` metric exposing the Gateway version and git revision
- Use a WebSocket load balancer to allow connections from behind NAT(s)
- `project_id` and `connection_string` can also be fetched from a file
- Required `database.pool_max_size` config value (overridable via `BLOCKFROST_GATEWAY_DB_POOL_MAX_SIZE`) that caps the PostgreSQL connection pool per gateway instance

### Fixed

- `GET /` no longer always reports `healthy: true` — it now reflects periodic health checks of PostgreSQL connectivity and the Blockfrost API
- The Gateway now refuses to start when the initial health check fails (e.g. an invalid Blockfrost project token)
- The underlying Blockfrost API error (e.g. rate limiting) is now logged when the license NFT check fails during registration

### Removed

- The TCP port reachability check during registration (and the associated `X-SKIP-PORT-CHECK` header / `SKIP_PORT_CHECK_SECRET` environment variable). It is no longer needed now that relays connect via the WebSocket load balancer.

## [1.3.3] - 2025-03-12

### Fixed

- HOTFIX: project_id env override

## [1.3.2] - 2025-03-12

### Added

- Check network and address mismatch
- Additional logging in registration

### Fixed

- Url in root route

## [1.3.1] - 2025-03-03

### Added

- Additional logging when checking IP addresses

### Changed

- Improved IP address validation and handling, including localhost scenarios.
- Nix devshell, checks, and package definition

### Fixed

- Better IPv6 support

## [1.3.0] - 2025-01-30

### Added

`asset_name` column in `requests` table that contains the asset name of the NFT

## [1.2.2] - 2025-01-24

### Changed

- Fetch api_prefix from blockfrost-platform

## [1.2.1] - 2025-01-24

### Fixed

- Digital Ocean ip address header

## [1.2.0] - 2025-01-24

### Added

- Check if server us accessible

### Fixed

- internal IP address -> external IP address

## [1.1.0] - 2025-01-23

### Added

- Config can be overridden by environment variables `SERVER_ADDRESS`, `SERVER_LOG_LEVEL`, `DB_CONNECTION_STRING`, `BLOCKFROST_PROJECT_ID`, `BLOCKFROST_NFT_ASSET`

### Fixed

- NFT checking

### Removed

- unused code

## [1.0.1] - 2024-11-14

### Fixed

- Pass route params on success response

### Added

- initial release
