//! The one registrar: the settings profile of each default, its cases, definitions, modules and evidence, staged and pinned.
use crate::outbound::artifacts::types::Sealed;
use crate::outbound::inventory::interface::InventoryPort;
use crate::outbound::suite::interface::SuitePort;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::{Path, PathBuf};

/// Stage every default of the bench at `bench` under `output` and write its registration there; answer the registration's path.
///
/// # Errors
/// Refuses an unreadable bench profile and a registration that cannot be written; a rule that cannot be staged is
/// registered with its reason.
pub fn register(
    suite: &dyn SuitePort,
    inventory: &dyn InventoryPort,
    bench: &Path,
    reference: &Sealed,
    worker: &Sealed,
    output: &Path,
) -> Result<PathBuf, ReportsError> {
    todo!(
        "register {} with {reference:?} and {worker:?} under {} through {suite:p} and {inventory:p}",
        bench.display(),
        output.display()
    )
}
