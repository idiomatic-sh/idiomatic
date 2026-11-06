//! Timed benchmarks require exactly their immutable declared rule identities.
use ryo::linter::RuleId;
use ryo_benchmark_contracts::domain::measurements::types::MeasurementMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;
/// Refuse narrowed, duplicate, missing or extra benchmark rules; diagnostics are explicit.
/// # Errors
/// Returns a refusal before either contestant runs when any inventory differs.
pub fn require_inventory(
    mode: &MeasurementMode,
    authored: &[RuleId],
    selected: Option<&[RuleId]>,
) -> Result<(), Error> {
    let MeasurementMode::Benchmark { inventory } = mode else {
        return Ok(());
    };
    let declared: std::collections::BTreeSet<_> = inventory.iter().collect();
    let authored_set: std::collections::BTreeSet<_> = authored.iter().collect();
    let selected = selected.ok_or_else(|| {
        Error::Invalid(
            crate::domain::measurements::constants::BENCHMARK_REQUIRES_EXPLICITLY_RESOLVED_COMPL
                .into(),
        )
    })?;
    let actual: std::collections::BTreeSet<_> = selected.iter().collect();
    if declared.is_empty()
        || declared.len() != inventory.len()
        || authored_set.len() != authored.len()
        || actual.len() != selected.len()
        || declared != authored_set
        || declared != actual
    {
        return Err(Error::Invalid(
            crate::domain::measurements::constants::BENCHMARK_RULE_IDENTITIES_DIFFER_FROM_DECLAR
                .into(),
        ));
    }
    Ok(())
}
