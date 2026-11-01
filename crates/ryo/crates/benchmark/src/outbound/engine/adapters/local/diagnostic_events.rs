//! Synchronous JSONL event output used only by explicit diagnostic worker runs.
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use tracing_subscriber::fmt::writer::MakeWriter;

/// Events describing one selected rule's work on a source file.
const RULE_FILE_TARGET: &str = "ryo::diagnostic::rule_file";
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
mod tests {
    use super::{DiagnosticEventWriter, RULE_FILE_TARGET, RULE_STEP_TARGET, subscriber};
    use std::io::Write as _;
    use tracing_subscriber::fmt::writer::MakeWriter as _;

    #[test]
    fn event_writer_flushes_complete_lines_for_live_diagnostics()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("events.jsonl");
        let writer = DiagnosticEventWriter::create(&path)?;
        {
            let mut event = writer.make_writer();
            event.write_all(b"{\"event\":\"rule_file_start\"}\n")?;
        }
        assert_eq!(
            std::fs::read_to_string(path)?,
            "{\"event\":\"rule_file_start\"}\n"
        );
        Ok(())
    }

    #[test]
    fn diagnostic_subscriber_writes_readable_jsonl_events() -> Result<(), Box<dyn std::error::Error>>
    {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("events.jsonl");
        let dispatch = tracing::Dispatch::new(subscriber(&path)?);
        tracing::dispatcher::with_default(&dispatch, || {
            tracing::trace!(
                target: RULE_FILE_TARGET,
                event = "rule_file_start",
                rule = "F821",
                file = "example.py",
            );
        });
        let output = std::fs::read_to_string(path)?;
        assert!(output.ends_with('\n'));
        assert!(output.contains("rule_file_start"));
        assert!(output.contains("F821"));
        assert!(output.contains("example.py"));
        Ok(())
    }

    #[test]
    fn diagnostic_subscriber_records_step_events_with_nested_context()
    -> Result<(), Box<dyn std::error::Error>> {
        let directory = tempfile::tempdir()?;
        let path = directory.path().join("events.jsonl");
        let dispatch = tracing::Dispatch::new(subscriber(&path)?);
        tracing::dispatcher::with_default(&dispatch, || {
            let span = tracing::trace_span!(
                target: RULE_STEP_TARGET,
                "rule_file",
                rule = "F811",
                file = "example.py",
            );
            let _entered = span.enter();
            let step_span = tracing::trace_span!(
                target: RULE_STEP_TARGET,
                "rule_step",
                invocation_id = 7_u64,
                step_index = 3_usize,
                parent_step_index = 2_usize,
                nested = true,
                operation = "join",
                operation_details = "Join { left: Step(1), right: Relation(Defines) }",
            );
            let _step_entered = step_span.enter();
            tracing::trace!(
                target: RULE_STEP_TARGET,
                event = "rule_step_start",
                invocation_id = 7_u64,
                step_index = 3,
                operation = "join",
            );
        });
        let output = std::fs::read_to_string(path)?;
        assert!(output.ends_with('\n'));
        assert!(output.contains("rule_step_start"));
        assert!(output.contains("F811"));
        assert!(output.contains("example.py"));
        assert!(output.contains("join"));
        assert!(output.contains("invocation_id"));
        assert!(output.contains("parent_step_index"));
        Ok(())
    }

    #[test]
    fn ordinary_info_subscriber_does_not_enable_diagnostic_step_events() {
        let subscriber = tracing_subscriber::fmt()
            .with_max_level(tracing::Level::INFO)
            .finish();
        let dispatch = tracing::Dispatch::new(subscriber);
        let enabled = tracing::dispatcher::with_default(
            &dispatch,
            || tracing::enabled!(target: RULE_STEP_TARGET, tracing::Level::TRACE),
        );
        assert!(!enabled);
    }
}
