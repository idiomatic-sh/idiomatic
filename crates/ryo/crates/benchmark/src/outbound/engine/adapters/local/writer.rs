//! Synchronous JSONL event writer used only by explicit diagnostic worker runs.
use crate::outbound::engine::adapters::local::constants;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};
use tracing_subscriber::fmt::writer::MakeWriter;

/// Events describing one selected rule's work on a source file.
pub(super) const RULE_FILE_TARGET: &str = constants::RYO_DIAGNOSTIC_RULE_FILE;
/// Compile-time source and normalized prefix evidence for explicit attribution.
pub(super) const COMPILED_PREFIX_TARGET: &str = constants::RYO_DIAGNOSTIC_COMPILED_PREFIX;
/// Complete compiled operations and file-specific physical layouts for an offline census.
pub(super) const COMPILED_SUBPLAN_TARGET: &str = constants::RYO_DIAGNOSTIC_COMPILED_SUBPLAN;
/// Completed kept-cut sizes and partial state for the same explicitly scoped census.
pub(super) const SUBPLAN_CUT_TARGET: &str = constants::RYO_DIAGNOSTIC_SUBPLAN_CUT;
/// Aggregate counters for an explicitly diagnosed generic stored join.
pub(super) const STORED_JOIN_TARGET: &str = constants::RYO_DIAGNOSTIC_STORED_JOIN;
/// Events describing one operator step within a selected rule.
pub(super) const RULE_STEP_TARGET: &str = constants::RYO_DIAGNOSTIC_RULE_STEP;
/// Aggregate phase timings for the explicitly requested execution profile.
pub(super) const EXECUTION_TARGET: &str =
    ryo_values::domain::table_words::constants::EXECUTION_TARGET;
/// Aggregate stored selection and materialization work.
pub(super) const SELECTION_TARGET: &str = constants::RYO_DIAGNOSTIC_SELECTION;
/// Per-prepared-selector counts, enabled only by an explicit diagnostic opt-in.
pub(super) const SELECTION_DETAIL_TARGET: &str = constants::RYO_DIAGNOSTIC_SELECTION_DETAIL;
/// Logical release and retained-capacity accounting.
pub(super) const WORKING_SET_TARGET: &str = constants::RYO_DIAGNOSTIC_WORKING_SET;
/// Metadata-only layout accounting.
pub(super) const LAYOUT_TARGET: &str = constants::RYO_DIAGNOSTIC_LAYOUT;
/// Demand-reach and shared graph work in explicit diagnostic runs.
pub(super) const GRAPH_TARGET: &str = constants::RYO_DIAGNOSTIC_GRAPH;
/// Request-wide file-program sharing and retained cells in explicit diagnostic runs.
pub(super) const FILE_PROGRAM_TARGET: &str = constants::RYO_DIAGNOSTIC_FILE_PROGRAM;
/// Coarse retention/decoding/store phases for an explicit lifecycle diagnostic.
pub(super) const RESOURCE_ROUTES_TARGET: &str =
    ryo_values::domain::table_words::constants::RESOURCE_ROUTES_TARGET;
pub(super) const LIFECYCLE_TARGET: &str = constants::RYO_DIAGNOSTIC_LIFECYCLE_MEMORY;

/// Shared append-only file writer; each event holds the lock until its JSON line is complete.
#[derive(Clone)]
pub(super) struct DiagnosticEventWriter(Arc<Mutex<File>>);

impl DiagnosticEventWriter {
    /// Share the already opened diagnostic stream with every event of the worker.
    pub(super) fn new(file: File) -> Self {
        Self(Arc::new(Mutex::new(file)))
    }
}

/// Create or truncate the explicit diagnostic stream before the worker begins.
pub(super) fn open(path: &Path) -> io::Result<File> {
    OpenOptions::new()
        .create(true)
        .truncate(true)
        .write(true)
        .open(path)
}

/// Build the filtered JSONL subscriber used by an explicit diagnostic worker.
pub(super) fn subscriber(
    path: &Path,
    options: ryo_benchmark_contracts::domain::requests::types::DiagnosticOptions,
) -> io::Result<impl tracing::Subscriber + Send + Sync + 'static> {
    let on = |switch: ryo_benchmark_contracts::domain::requests::types::DiagnosticSwitch| {
        matches!(
            switch,
            ryo_benchmark_contracts::domain::requests::types::DiagnosticSwitch::On
        )
    };
    let aggregate_level = if on(options.execution) && !on(options.lifecycle_memory) {
        tracing_subscriber::filter::LevelFilter::TRACE
    } else {
        tracing_subscriber::filter::LevelFilter::OFF
    };
    let lifecycle_only = on(options.lifecycle_memory);
    let resource_routes = on(options.resource_routes);
    let lifecycle_level = if lifecycle_only {
        tracing_subscriber::filter::LevelFilter::TRACE
    } else {
        aggregate_level
    };
    let writer = DiagnosticEventWriter::new(open(path)?);
    let progress_only = on(options.row_progress);
    let include_steps = on(options.steps) && !lifecycle_only && !progress_only;
    let include_joins = on(options.joins) && !lifecycle_only;
    let include_prefixes = on(options.prefixes) && !lifecycle_only;
    let include_subplans = on(options.subplans) && !lifecycle_only;
    let include_selection_detail = on(options.selection_detail) && !lifecycle_only;
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
                    constants::RYO_DIAGNOSTIC_ROW_PROGRESS,
                    if on(options.row_progress) {
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
