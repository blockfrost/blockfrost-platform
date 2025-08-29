## Unreleased

### Added

- Support for optional `X-SKIP-PORT-CHECK` header. When set to the value of the `SKIP_PORT_CHECK_SECRET` environment variable, the server skips the TCP port reachability check and accepts the provided port as-is.

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
