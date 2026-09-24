# Copilot instructions for blockfrost-platform

This is a Rust workspace (monorepo). Crates live under `crates/` — notably
`crates/gateway` (the WebSocket load-balancing gateway) and `crates/platform`.

## Changelog (required for user-facing changes)

The project keeps a single [Keep a Changelog](https://keepachangelog.com)-style
file:

- Root [`CHANGELOG.md`](../CHANGELOG.md) — product-wide, user-facing changes for
  the whole platform, including the gateway. Everything shares Platform's
  versioning scheme; the gateway no longer has its own changelog or version
  numbers. Scope gateway-specific entries with a `Gateway:` prefix.

New entries go under the top `## [Unreleased]` heading, inside the matching
`### Added`, `### Changed`, or `### Fixed` subsection.

**When reviewing a pull request, flag a missing changelog entry** if the PR:

- adds a feature (new HTTP endpoint, new config option, new CLI flag), or
- changes user-facing behavior, or
- fixes a user-visible bug,

…but does not add a corresponding entry under `## [Unreleased]` in
`CHANGELOG.md`. Say which `###` section the entry belongs in, and suggest a
one-line entry (prefixed with `Gateway:` if it's a gateway change).

Documentation-only, test-only, refactor, and CI/tooling changes do not need a
changelog entry.
