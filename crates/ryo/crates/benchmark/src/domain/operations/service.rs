//! The verbs of the operations, the directory each writes and the worker command each runs.
use super::constants::{
    BENCH, BENCH_COMPONENTS, BENCHMARK, BENCHMARKS, COLLECTOR, DOCUMENT, INVENTORY, LANGUAGES,
    LINTERS, NOT_A_BENCH_DIRECTORY, REGISTRATION, STATE,
};
use super::types::{BenchDirectory, BenchOperation, OperationLayout, OperationVerb};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use ryo_benchmark_contracts::domain::requests::constants::{
    CASE_INVENTORY_ARGUMENT, DOCUMENT_ARGUMENT, MEASURE_ACCOUNTED_VALID, VERIFICATION_ARGUMENT,
};
use std::ffi::OsString;
use std::path::{Component, Path};

/// The operation a collector verb names.
#[must_use]
pub fn operation(verb: OperationVerb<'_>) -> Option<BenchOperation> {
    [BenchOperation::Bench]
        .into_iter()
        .find(|operation| verb_of(*operation).0 == verb.0)
}

/// The collector verb of an operation.
pub(crate) fn verb_of(operation: BenchOperation) -> OperationVerb<'static> {
    OperationVerb(match operation {
        BenchOperation::Bench => BENCH,
    })
}

/// Where an operation of a bench writes, under the cargo target: `<target>/benchmark/<lang>/<linter>/<verb>`.
///
/// # Errors
/// Refuses a path that does not end in `languages/<lang>/benchmarks/linters/<linter>`.
pub fn layout(
    target: &Path,
    bench: &BenchDirectory,
    operation: BenchOperation,
) -> Result<OperationLayout, ReportsError> {
    let names = bench
        .0
        .components()
        .map(|component| match component {
            Component::Normal(name) => name.to_str(),
            Component::Prefix(_)
            | Component::RootDir
            | Component::CurDir
            | Component::ParentDir => None,
        })
        .collect::<Vec<_>>();
    match names.get(names.len().saturating_sub(BENCH_COMPONENTS)..) {
        Some(
            [
                Some(LANGUAGES),
                Some(language),
                Some(BENCHMARKS),
                Some(LINTERS),
                Some(linter),
            ],
        ) => Ok(OperationLayout {
            root: target
                .join(BENCHMARK)
                .join(language)
                .join(linter)
                .join(verb_of(operation).0),
        }),
        Some(_) | None => Err(ReportsError::Invalid(NOT_A_BENCH_DIRECTORY.into())),
    }
}

/// The file the worker writes the bench's one document to: `<layout>/document.bmf`.
pub(crate) fn document(layout: &OperationLayout) -> std::path::PathBuf {
    layout.root.join(DOCUMENT)
}

/// The arguments the sealed worker of a bench runs its corpus with: `measure-valid`, certified against the registration,
/// reading the times the verification left and writing the bench's one document to `document`.
pub(crate) fn worker_arguments(
    operation: BenchOperation,
    layout: &OperationLayout,
    tool: &Path,
) -> Vec<OsString> {
    match operation {
        BenchOperation::Bench => {
            let mut inventory = OsString::from(CASE_INVENTORY_ARGUMENT);
            inventory.push(layout.root.join(REGISTRATION));
            let mut verification = OsString::from(VERIFICATION_ARGUMENT);
            verification.push(layout.root.join(INVENTORY));
            let mut written = OsString::from(DOCUMENT_ARGUMENT);
            written.push(document(layout));
            vec![
                MEASURE_ACCOUNTED_VALID.into(),
                tool.into(),
                layout.root.join(STATE).into(),
                layout.root.join(COLLECTOR).into(),
                inventory,
                verification,
                written,
            ]
        }
    }
}
