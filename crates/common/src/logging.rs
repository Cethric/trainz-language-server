use tracing::trace;
use tracing_subscriber::{EnvFilter, fmt};

pub use tracing_subscriber;
pub use tracing_subscriber::fmt::writer::BoxMakeWriter;

/// Initializes the logger with the specified level and writer.
pub fn setup_logger(level: Option<String>, writer: Option<BoxMakeWriter>) {
    let mut filter = EnvFilter::from_default_env();
    if let Some(level) = level {
        filter = filter.add_directive(level.parse().unwrap());
    }

    let builder = fmt()
        .with_env_filter(filter)
        .with_target(true)
        .with_thread_ids(true)
        .with_thread_names(true)
        .with_file(true)
        .with_line_number(true)
        .with_ansi_sanitization(true)
        .with_ansi(false);

    if let Some(writer) = writer {
        builder.with_writer(writer).init();
    } else {
        builder.with_writer(std::io::stderr).init();
    }

    trace!("Logger initialized (tracing-subscriber)")
}
