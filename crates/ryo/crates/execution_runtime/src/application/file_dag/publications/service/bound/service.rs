//! What a consumer's bound observations answer once bound: each binding's rows, their coverage and where they come from, or
//! why it is withheld; and whether the preparation admitted the binding at all.

use crate::application::file_dag::program::preparation::types::ObservationMetadata;
use crate::application::file_dag::publications::types::{BoundRows, BoundTable, BoundTables};
use ryo_execution_domain::domain::completion::types::CoverageState;
use ryo_execution_domain::domain::coverage::types::RuleIndex;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_operators::domain::contract::types::WithheldInput;
use ryo_operators::domain::physical_plan::types::bound_observation_place::value::BoundObservationPlace;
use ryo_operators::domain::physical_plan::types::held_rows::value::HeldRows;
use ryo_values::domain::lineage::types::Lineage;
use ryo_values::domain::plan_parts::types::ObservationBindingId;

/// Whether `rule`'s binding `input` was admitted against its consumer's declaration when the program
/// was prepared.
///
/// # Errors
///
/// The refusal its admission answered.
pub(crate) fn admitted(
    metadata: &ObservationMetadata,
    rule: RuleIndex,
    input: ObservationBindingId,
) -> Result<(), OperatingError> {
    let admission = metadata
        .bindings
        .get(rule.0)
        .and_then(|bindings| bindings.iter().position(|binding| binding.input == input))
        .and_then(|place| {
            metadata
                .admissions
                .get(rule.0)
                .and_then(|admissions| admissions.get(place))
        });
    match admission {
        Some(admission) => admission.clone(),
        None => Err(OperatingError::NotRun),
    }
}

/// The rows a bound observation of `place`'s rule answers, or the token of why they are withheld.
pub(crate) fn held_observation(bound: &BoundTables, place: BoundObservationPlace) -> HeldRows<'_> {
    match bound.0.iter().find(|table| table.input == place.input) {
        Some(BoundTable {
            rows: BoundRows::Held { table, .. },
            ..
        }) => HeldRows::Held(table),
        Some(BoundTable {
            rows: BoundRows::Withheld(token),
            ..
        }) => HeldRows::Withheld(*token),
        None => HeldRows::Withheld(WithheldInput(u32::MAX)),
    }
}

/// Where a bound observation's rows come from, when they are held.
pub(crate) fn bound_lineage(bound: &BoundTables, input: ObservationBindingId) -> Option<&Lineage> {
    bound.0.iter().find_map(|table| match table {
        BoundTable {
            input: found,
            rows: BoundRows::Held { lineage, .. },
        } if *found == input => Some(lineage),
        BoundTable { .. } => None,
    })
}

/// The coverage of a bound observation's rows, when they are held.
pub(crate) fn bound_coverage(
    bound: &BoundTables,
    input: ObservationBindingId,
) -> Option<&CoverageState> {
    bound.0.iter().find_map(|table| match table {
        BoundTable {
            input: found,
            rows: BoundRows::Held { coverage, .. },
        } if *found == input => Some(coverage),
        BoundTable { .. } => None,
    })
}
