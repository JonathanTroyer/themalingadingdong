//! Logging configuration using tracing with file appender.

use std::path::Path;

use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::prelude::*;
use tracing_subscriber::{EnvFilter, fmt};

/// Initialize tracing with file output.
///
/// Returns a guard that must be held for the duration of the program to ensure
/// logs are flushed. Dropping the guard flushes remaining logs.
///
/// In debug builds, span enter/exit events are logged for detailed tracing.
/// In release builds, only explicit log events are recorded for performance.
pub fn init_logging(log_path: Option<&Path>, level: Option<&str>) -> WorkerGuard {
    let log_path = log_path.unwrap_or(Path::new("themalingadingdong.log"));
    let level = level.unwrap_or("info");

    let parent = log_path.parent().unwrap_or(Path::new("."));
    let filename = log_path
        .file_name()
        .unwrap_or_else(|| std::ffi::OsStr::new("themalingadingdong.log"));

    let file_appender = tracing_appender::rolling::never(parent, filename);
    let (non_blocking, guard) = tracing_appender::non_blocking(file_appender);

    let filter = EnvFilter::try_new(format!("themalingadingdong={level}"))
        .unwrap_or_else(|_| EnvFilter::new("info"));

    let file_layer = fmt::layer()
        .with_writer(non_blocking)
        .with_ansi(false)
        .with_target(true)
        .with_level(true)
        .with_thread_ids(false);

    // Only add span events in debug builds (significant overhead in release)
    #[cfg(debug_assertions)]
    let file_layer = {
        use tracing_subscriber::fmt::format::FmtSpan;
        file_layer.with_span_events(FmtSpan::ENTER | FmtSpan::CLOSE)
    };

    tracing_subscriber::registry()
        .with(filter)
        .with(file_layer)
        .init();

    guard
}
