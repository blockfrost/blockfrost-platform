use std::fmt::{self, Write as _};
use tracing::{Event, Level, Subscriber};
use tracing_subscriber::fmt::format::Format;
use tracing_subscriber::fmt::{FmtContext, FormatEvent, FormatFields, FormattedFields, format};
use tracing_subscriber::registry::LookupSpan;

/// A [`fmt::Write`] adapter that inserts `prefix` after every embedded `\n`,
/// so that each output line carries the syslog priority tag.
struct LinePrefixer<'a, W> {
    inner: W,
    prefix: &'a str,
}

impl<W: fmt::Write> fmt::Write for LinePrefixer<'_, W> {
    fn write_str(&mut self, s: &str) -> fmt::Result {
        let mut rest = s;
        while let Some(nl) = rest.find('\n') {
            self.inner.write_str(&rest[..=nl])?;
            self.inner.write_str(self.prefix)?;
            rest = &rest[nl + 1..];
        }
        self.inner.write_str(rest)
    }
}

/// A log event formatter that prepends syslog priority prefixes (`<N>`) to each line.
pub struct SyslogFormat;

impl<S, N> FormatEvent<S, N> for SyslogFormat
where
    S: Subscriber + for<'a> LookupSpan<'a>,
    N: for<'a> FormatFields<'a> + 'static,
{
    fn format_event(
        &self,
        ctx: &FmtContext<'_, S, N>,
        mut writer: format::Writer<'_>,
        event: &Event<'_>,
    ) -> fmt::Result {
        let metadata = event.metadata();
        let level = metadata.level();

        let prefix: &str = match *level {
            Level::ERROR => "<3>",
            Level::WARN => "<4>",
            Level::INFO => "<6>",
            _ => "<7>",
        };

        writer.write_str(prefix)?;

        {
            let mut prefixer = LinePrefixer {
                inner: writer.by_ref(),
                prefix,
            };

            write!(prefixer, "{}: ", metadata.target())?;

            {
                let pw = format::Writer::new(&mut prefixer);
                ctx.format_fields(pw, event)?;
            }

            if let Some(scope) = ctx.event_scope() {
                for span in scope.from_root() {
                    let ext = span.extensions();
                    if let Some(fields) = ext.get::<FormattedFields<N>>()
                        && !fields.is_empty()
                    {
                        write!(prefixer, " {fields}")?;
                    }
                }
            }
        }

        writeln!(writer)
    }
}

/// Sets up the tracing subscriber.
///
/// Reads the environment variable named `log_target_env` to determine the output mode:
///
/// - `journal` – native journald transport via `tracing-journald` (Linux only)
/// - `syslog` – syslog priority prefixes on stdout, suitable for journald
///   ingestion via `SyslogLevelPrefix=yes` (the default)
/// - otherwise – default colored compact format
///
/// When `log_target_env` is unset but `$JOURNAL_STREAM` is present (i.e. the
/// process was started by systemd with stdout/stderr connected to the journal),
/// the mode defaults to `syslog` so that journald can parse priority levels.
/// Determines the logging target from two optional env var values.
///
/// Returns the explicit target when set and non-empty, falls back to `"syslog"`
/// when `journal_stream` is present, or `None` for the default colored output.
fn resolve_log_target(
    log_target_value: Option<String>,
    journal_stream: Option<String>,
) -> Option<String> {
    log_target_value
        .filter(|s| !s.trim().is_empty())
        .or_else(|| journal_stream.map(|_| "syslog".into()))
}

pub fn setup_tracing(log_level: Level, log_target_env: &str) {
    let log_target = resolve_log_target(
        std::env::var(log_target_env).ok(),
        std::env::var("JOURNAL_STREAM").ok(),
    );

    match log_target.as_deref() {
        #[cfg(target_os = "linux")]
        Some("journal") => {
            use tracing_journald::{Priority, PriorityMappings};
            use tracing_subscriber::prelude::*;
            let journald_layer = tracing_journald::layer()
                .expect("Failed to connect to systemd journal socket")
                .with_priority_mappings(PriorityMappings {
                    info: Priority::Informational,
                    debug: Priority::Debug,
                    ..PriorityMappings::new()
                });
            tracing_subscriber::registry()
                .with(journald_layer)
                .with(tracing_subscriber::filter::LevelFilter::from_level(
                    log_level,
                ))
                .init();
        },
        #[cfg(not(target_os = "linux"))]
        Some("journal") => {
            eprintln!(
                "{log_target_env}=journal is only supported on Linux, \
                 falling back to default logging"
            );
            setup_default(log_level);
        },
        Some("syslog") => {
            tracing_subscriber::fmt()
                .with_max_level(log_level)
                .event_format(SyslogFormat)
                .init();
        },
        _ => {
            setup_default(log_level);
        },
    }
}

fn setup_default(log_level: Level) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use rstest::rstest;

    fn s(v: &str) -> Option<String> {
        Some(v.to_owned())
    }

    #[rstest]
    #[case::both_unset(None, None, None)]
    #[case::journal_stream_only(None, s("8:12345"), Some("syslog"))]
    #[case::explicit_journal(s("journal"), None, Some("journal"))]
    #[case::explicit_syslog(s("syslog"), None, Some("syslog"))]
    #[case::empty_treated_as_unset(s(""), None, None)]
    #[case::whitespace_treated_as_unset(s("  "), None, None)]
    #[case::empty_falls_through_to_journal(s(""), s("8:12345"), Some("syslog"))]
    #[case::whitespace_falls_through_to_journal(s(" \t "), s("8:12345"), Some("syslog"))]
    #[case::explicit_overrides_journal_stream(s("journal"), s("8:12345"), Some("journal"))]
    fn resolve_log_target_cases(
        #[case] target: Option<String>,
        #[case] journal: Option<String>,
        #[case] expected: Option<&str>,
    ) {
        assert_eq!(
            resolve_log_target(target, journal),
            expected.map(String::from),
        );
    }
}
