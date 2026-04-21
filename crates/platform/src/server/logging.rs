use tracing::Level;
use tracing_subscriber::fmt::format::Format;

/// Sets up the tracing subscriber with the provided configuration.
pub fn setup_tracing(log_level: Level) {
    #[cfg(target_os = "linux")]
    if std::env::var("BLOCKFROST_PLATFORM_LOG_TARGET")
        .ok()
        .as_deref()
        == Some("journal")
    {
        if let Ok(journald_layer) = tracing_journald::layer() {
            use tracing_subscriber::prelude::*;
            tracing_subscriber::registry()
                .with(journald_layer)
                .with(tracing_subscriber::filter::LevelFilter::from_level(
                    log_level,
                ))
                .init();
            return;
        }
        eprintln!(
            "WARNING: BLOCKFROST_PLATFORM_LOG_TARGET=journal but failed to connect to journald, falling back to stdout"
        );
    }

    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .event_format(
            Format::default()
                .with_ansi(true)
                .with_level(true)
                .with_target(true)
                .compact(),
        )
        .init();
}
