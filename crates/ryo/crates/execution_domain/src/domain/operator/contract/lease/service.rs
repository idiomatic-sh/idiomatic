//! The `lease` service: free functions named by verb over the domain's types, with no IO and no ports.

use super::super::types::{
    DistinctWorkspace, FoundRows, PrimitiveLease, PrimitiveViews, PrimitiveWorkspace, WorkCounters,
    WorkTotals,
};
use super::super::views::types::PredicateProgram;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::PrimitiveSemantics;
use ryo_language::domain::symbols::types::QuerySymbols;
use ryo_relations::domain::relations::types::Value;

/// Adds the rows an input was found to hold.
pub fn add_found(
    counters: &WorkCounters,
    input: ryo_values::domain::plan_parts::types::InputNumber,
    rows: FoundRows,
) {
    if let Some(total) = counters.0.get(input.0) {
        total.set(total.get().saturating_add(rows.0));
    }
}

/// The totals counted so far, resetting the counters.
pub fn take_totals(counters: &WorkCounters) -> WorkTotals {
    WorkTotals(std::array::from_fn(|at| {
        FoundRows(counters.0[at].replace(0))
    }))
}

pub(crate) fn distinct<'a, 'r>(
    lease: &'r mut PrimitiveLease<'a>,
) -> Result<&'r mut DistinctWorkspace, OperatingError> {
    let state = lease
        .workspace
        .distinct
        .as_mut()
        .ok_or(OperatingError::WrongBinding)?;
    state.schema.as_ref().map_err(Clone::clone)?;
    Ok(state)
}

#[cfg(feature = "diagnostic-row-progress")]
pub(crate) fn progress<'a>(
    lease: &PrimitiveLease<'a>,
    counter: crate::domain::operating::types::ProgressCounter,
    amount: crate::domain::operating::types::ProgressAmount,
) {
    lease.semantics.progress().observe(counter, amount);
}

/// Copy the request-lived semantic port before borrowing this lease for row delivery.
pub(crate) fn semantics<'a>(lease: &PrimitiveLease<'a>) -> &'a dyn PrimitiveSemantics {
    lease.semantics
}

pub(crate) fn primitive_lease<'a>(
    views: PrimitiveViews<'a>,
    workspace: &'a mut PrimitiveWorkspace,
    derived: QuerySymbols<'a>,
    predicate: &'a mut dyn PredicateProgram,
    semantics: &'a dyn PrimitiveSemantics,
    work: &'a WorkCounters,
) -> PrimitiveLease<'a> {
    PrimitiveLease {
        views,
        workspace,
        derived,
        predicate,
        semantics,
        work,
    }
}

pub(crate) fn found<'a>(
    lease: &PrimitiveLease<'a>,
    input: ryo_values::domain::plan_parts::types::InputNumber,
    rows: FoundRows,
) {
    add_found(lease.work, input, rows);
}

/// Only the bound semantic program can be requested by a primitive.
pub(crate) fn holds<'a>(
    lease: &mut PrimitiveLease<'a>,
    row: &[Value],
) -> Result<crate::domain::table_model::types::Verdict, OperatingError> {
    lease.predicate.holds(row, lease.derived.as_ref())
}
