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
/// Complete compiled operations and file-specific physical layouts for an offline census.
const COMPILED_SUBPLAN_TARGET: &str = "ryo::diagnostic::compiled_subplan";
/// Completed kept-cut sizes and partial state for the same explicitly scoped census.
const SUBPLAN_CUT_TARGET: &str = "ryo::diagnostic::subplan_cut";
/// Aggregate counters for an explicitly diagnosed generic stored join.
const STORED_JOIN_TARGET: &str = "ryo::diagnostic::stored_join";
/// Events describing one operator step within a selected rule.
const RULE_STEP_TARGET: &str = "ryo::diagnostic::rule_step";
/// Aggregate phase timings for the explicitly requested execution profile.
const EXECUTION_TARGET: &str = "ryo::diagnostic::execution";
/// Aggregate stored selection and materialization work.
const SELECTION_TARGET: &str = "ryo::diagnostic::selection";
/// Per-prepared-selector counts, enabled only by an explicit diagnostic opt-in.
const SELECTION_DETAIL_TARGET: &str = "ryo::diagnostic::selection_detail";
/// Logical release and retained-capacity accounting.
const WORKING_SET_TARGET: &str = "ryo::diagnostic::working_set";
/// Metadata-only layout accounting.
const LAYOUT_TARGET: &str = "ryo::diagnostic::layout";
/// Demand-reach and shared graph work in explicit diagnostic runs.
const GRAPH_TARGET: &str = "ryo::diagnostic::graph";
/// Request-wide file-program sharing and retained cells in explicit diagnostic runs.
const FILE_PROGRAM_TARGET: &str = "ryo::diagnostic::file_program";
/// Coarse retention/decoding/store phases for an explicit lifecycle diagnostic.
const RESOURCE_ROUTES_TARGET: &str = "ryo::diagnostic::resource_routes";
const LIFECYCLE_TARGET: &str = "ryo::diagnostic::lifecycle_memory";

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
    options: crate::domain::suite::types::DiagnosticOptions,
) -> io::Result<impl tracing::Subscriber + Send + Sync + 'static> {
    let aggregate_level = if options.execution && !options.lifecycle_memory {
        tracing_subscriber::filter::LevelFilter::TRACE
    } else {
        tracing_subscriber::filter::LevelFilter::OFF
    };
    let lifecycle_only = options.lifecycle_memory;
    let resource_routes = options.resource_routes;
    let lifecycle_level = if lifecycle_only {
        tracing_subscriber::filter::LevelFilter::TRACE
    } else {
        aggregate_level
    };
    let writer = DiagnosticEventWriter::create(path)?;
    let progress_only = options.row_progress;
    let include_steps = options.steps && !lifecycle_only && !progress_only;
    let include_joins = options.joins && !lifecycle_only;
    let include_prefixes = options.prefixes && !lifecycle_only;
    let include_subplans = options.subplans && !lifecycle_only;
    let include_selection_detail = options.selection_detail && !lifecycle_only;
    use tracing_subscriber::Layer as _;
    use tracing_subscriber::layer::SubscriberExt as _;
    let layer = tracing_subscriber::fmt::layer()
        .json()
        .with_ansi(false)
        .with_current_span(true)
        .with_writer(writer)
        .with_filter(
            tracing_subscriber::filter::Targets::new()
                .with_target(
                    RESOURCE_ROUTES_TARGET,
                    if resource_routes && !lifecycle_only {
                        tracing_subscriber::filter::LevelFilter::TRACE
                    } else {
                        tracing_subscriber::filter::LevelFilter::OFF
                    },
                )
                .with_target(
                    "ryo::diagnostic::row_progress",
                    if options.row_progress {
                        tracing_subscriber::filter::LevelFilter::TRACE
                    } else {
                        tracing_subscriber::filter::LevelFilter::OFF
                    },
                )
                .with_target(EXECUTION_TARGET, aggregate_level)
                .with_target(SELECTION_TARGET, aggregate_level)
                .with_target(
                    SELECTION_DETAIL_TARGET,
                    if include_selection_detail {
                        tracing_subscriber::filter::LevelFilter::TRACE
                    } else {
                        tracing_subscriber::filter::LevelFilter::OFF
                    },
                )
                .with_target(WORKING_SET_TARGET, aggregate_level)
                .with_target(LAYOUT_TARGET, aggregate_level)
                .with_target(GRAPH_TARGET, aggregate_level)
                .with_target(FILE_PROGRAM_TARGET, aggregate_level)
                .with_target(LIFECYCLE_TARGET, lifecycle_level)
                .with_target(
                    RULE_FILE_TARGET,
                    if lifecycle_only || progress_only || !include_steps {
                        tracing_subscriber::filter::LevelFilter::OFF
                    } else {
                        tracing_subscriber::filter::LevelFilter::TRACE
                    },
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
                    COMPILED_SUBPLAN_TARGET,
                    if include_subplans {
                        tracing_subscriber::filter::LevelFilter::TRACE
                    } else {
                        tracing_subscriber::filter::LevelFilter::OFF
                    },
                )
                .with_target(
                    SUBPLAN_CUT_TARGET,
                    if include_subplans {
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
