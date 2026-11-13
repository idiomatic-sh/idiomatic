//! The verbs of the operations, the directory each writes and the worker command each runs.
use super::constants::{
    BENCH_COMPONENTS, BENCHMARK, BENCHMARKS, CASE_INVENTORY, COLLECTOR, LANGUAGES, LINTERS,
    NOT_A_BENCH_DIRECTORY, PARITY, REGISTRATION, STATE, VERIFY_INVENTORY,
};
use super::types::{BenchDirectory, Operation, OperationLayout};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use ryo_benchmark_contracts::domain::requests::constants::MEASURE_ACCOUNTED_VALID;
use std::ffi::OsString;
use std::path::{Component, Path};

/// The operation a collector verb names.
#[must_use]
pub fn operation(verb: &str) -> Option<Operation> {
    [Operation::Correctness, Operation::Measure, Operation::Parity]
        .into_iter()
        .find(|operation| verb_of(*operation) == verb)
}

/// The collector verb of an operation.
pub(crate) fn verb_of(operation: Operation) -> &'static str {
    match operation {
        Operation::Correctness => VERIFY_INVENTORY,
        Operation::Measure => MEASURE_ACCOUNTED_VALID,
        Operation::Parity => PARITY,
    }
}

/// Where an operation of a bench writes, under the cargo target: `<target>/benchmark/<lang>/<linter>/<verb>`.
///
/// # Errors
/// Refuses a path that does not end in `languages/<lang>/benchmarks/linters/<linter>`.
pub fn layout(
    target: &Path,
    bench: &BenchDirectory,
    operation: Operation,
) -> Result<OperationLayout, ReportsError> {
    let names = bench
        .0
        .components()
        .map(|component| match component {
            Component::Normal(name) => name.to_str(),
            Component::Prefix(_) | Component::RootDir | Component::CurDir | Component::ParentDir => {
                None
            }
        })
        .collect::<Vec<_>>();
    match names.get(names.len().saturating_sub(BENCH_COMPONENTS)..) {
        Some([Some(LANGUAGES), Some(language), Some(BENCHMARKS), Some(LINTERS), Some(linter)]) => {
            Ok(OperationLayout {
                root: target
                    .join(BENCHMARK)
                    .join(language)
                    .join(linter)
                    .join(verb_of(operation)),
            })
        }
        Some(_) | None => Err(ReportsError::Invalid(NOT_A_BENCH_DIRECTORY.into())),
    }
}

/// The arguments the sealed worker of a bench runs an operation with; the correctness operation runs no worker.
pub(crate) fn worker_arguments(
    operation: Operation,
    layout: &OperationLayout,
    tool: &Path,
) -> Vec<OsString> {
    match operation {
        Operation::Correctness => Vec::new(),
        Operation::Measure => {
            let mut inventory = OsString::from(CASE_INVENTORY);
            inventory.push(layout.root.join(REGISTRATION));
            vec![
                MEASURE_ACCOUNTED_VALID.into(),
                tool.into(),
                layout.root.join(STATE).into(),
                layout.root.join(COLLECTOR).into(),
                inventory,
            ]
        }
        Operation::Parity => vec![PARITY.into(), tool.into(), layout.root.clone().into()],
    }
}
