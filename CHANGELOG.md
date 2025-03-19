## Unreleased

### Added

- Header `blockfrost-platform-response` in `tx_submit` endpoint
- Add `aarch64-linux` builds to release artifacts and installers.

### Fixed

- Node connections are now invalidated on unexpected transaction submission errors.
- Node connection metrics inconsistency caused by an initialization timing issue.
- Configure local IP address to bind to with `std::net` types.

## [0.0.1] - 2025-02-13

### Added

- Initial release
