use tracing::Level;
use tracing_subscriber::fmt::format::Format;

/// Sets up the tracing subscriber with the provided configuration.
pub fn setup_tracing(log_level: Level) {
    tracing_subscriber::fmt()
        .with_max_level(log_level)
        .event_format(
            Format::default()
                .with_ansi(true)
                .with_level(true)
                .with_target(false)
                .compact(),
        )
        .init();
}
