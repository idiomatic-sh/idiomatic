//! Synchronous JSONL event output used only by explicit diagnostic worker runs.
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use tracing_subscriber::fmt::writer::MakeWriter;

/// Events describing one selected rule's work on a source file.
const RULE_FILE_TARGET: &str = "ryo::diagnostic::rule_file";
/// Compile-time source and normalized prefix evidence for explicit attribution.
const COMPILED_PREFIX_TARGET: &str = "ryo::diagnostic::compiled_prefix";
/// Aggregate counters for an explicitly diagnosed generic stored join.
const STORED_JOIN_TARGET: &str = "ryo::diagnostic::stored_join";
/// Events describing one operator step within a selected rule.
const RULE_STEP_TARGET: &str = "ryo::diagnostic::rule_step";

/// Shared append-only file writer; each event holds the lock until its JSON line is complete.
#[derive(Clone)]
pub(super) struct DiagnosticEventWriter(Arc<Mutex<File>>);

impl DiagnosticEventWriter {
    /// Create or truncate the explicit diagnostic stream before the worker begins.
    pub(super) fn create(path: &Path) -> io::Result<Self> {
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)?;
        Ok(Self(Arc::new(Mutex::new(file))))
    }
}

/// Build the filtered JSONL subscriber used by an explicit diagnostic worker.
pub(super) fn subscriber(
    path: &Path,
) -> io::Result<impl tracing::Subscriber + Send + Sync + 'static> {
    use tracing_subscriber::Layer as _;
    use tracing_subscriber::layer::SubscriberExt as _;
    let writer = DiagnosticEventWriter::create(path)?;
    let include_steps = std::env::var_os("RYO_DIAGNOSTIC_STEPS").is_none_or(|value| value != "0");
    let include_joins = std::env::var_os("RYO_DIAGNOSTIC_JOINS").is_some_and(|value| value == "1");
    let include_prefixes =
        std::env::var_os("RYO_DIAGNOSTIC_PREFIXES").is_some_and(|value| value == "1");
    let layer = tracing_subscriber::fmt::layer()
        .json()
        .with_ansi(false)
        .with_current_span(true)
        .with_writer(writer)
        .with_filter(
            tracing_subscriber::filter::Targets::new()
                .with_target(
                    RULE_FILE_TARGET,
                    tracing_subscriber::filter::LevelFilter::TRACE,
                )
                .with_target(
                    COMPILED_PREFIX_TARGET,
                    if include_prefixes {
                        tracing_subscriber::filter::LevelFilter::TRACE
                    } else {
                        tracing_subscriber::filter::LevelFilter::OFF
                    },
                )
                .with_target(
                    STORED_JOIN_TARGET,
                    if include_joins {
                        tracing_subscriber::filter::LevelFilter::TRACE
                    } else {
                        tracing_subscriber::filter::LevelFilter::OFF
                    },
                )
                .with_target(
                    RULE_STEP_TARGET,
                    if include_steps {
                        tracing_subscriber::filter::LevelFilter::TRACE
                    } else {
                        tracing_subscriber::filter::LevelFilter::OFF
                    },
                ),
        );
    Ok(tracing_subscriber::registry().with(layer))
}

impl<'a> MakeWriter<'a> for DiagnosticEventWriter {
    type Writer = LockedDiagnosticEventWriter<'a>;

    fn make_writer(&'a self) -> Self::Writer {
        let file = self
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        LockedDiagnosticEventWriter { file }
    }
}

/// Holds one event's file lock through formatting, then flushes that complete line on drop.
pub(super) struct LockedDiagnosticEventWriter<'a> {
    /// Exclusive ownership of the stream through one complete event.
    file: MutexGuard<'a, File>,
}

impl Write for LockedDiagnosticEventWriter<'_> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        self.file.write(buf)
    }

    fn flush(&mut self) -> io::Result<()> {
        self.file.flush()
    }
}

impl Drop for LockedDiagnosticEventWriter<'_> {
    fn drop(&mut self) {
        let _flush_result = self.file.flush();
    }
}

#[cfg(test)]
#[path = "diagnostic_events_test.rs"]
mod tests;
