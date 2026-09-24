pub mod cardano_keys;
pub mod errors;
pub mod find_libexec;
pub mod hydra;
pub mod json_client;
pub mod pagination;
pub mod shutdown;
pub mod tcp_mux_tunnel;
pub mod tracing;
pub mod types;

/// Default maximum size (in bytes) of an HTTP body buffered when proxying
/// requests/responses between the platform, gateway, and SDK bridge.
pub const DEFAULT_MAX_BODY_BYTES: usize = 10 * 1024 * 1024;
