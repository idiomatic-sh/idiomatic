//! The `growth` concept's pure functions: the run budget's one check of a unit's makings of a node, and the one order of
//! makings by how far they fan out.
use super::constants::RUN_BUDGET;
use super::errors::GrowthError;
use super::types::{Growth, Making};
use ryo_values::domain::rows::types::RowCount;
use std::cmp::Ordering;

/// Whether a unit's makings of one node, of an operator whose instance grows as `growth`, fed `fed` rows and writing `wrote`
/// rows, are within the run budget. The unit is whatever its caller makes at once: a file today.
///
/// # Errors
///
/// `OverBudget` with the rows fed and written: when, of an operator not bounded by the text it reads, they wrote more than
/// the budget allows for the rows they were fed and more than the budget's floor, or when their operator is declared to write
/// at most its input and they wrote more than they were fed.
pub(crate) fn within_budget(growth: Growth, fed: RowCount, wrote: RowCount) -> Result<(), GrowthError> {
    let allowed = fed.0.max(1).saturating_mul(RUN_BUDGET.per_fed_row.0);
    // A text map writes rows by the text it reads, not by its rows: its bound is the file's bytes, which the growth tests
    // hold it to, so the rows per row fed do not bound it.
    let past_budget = growth != Growth::PerRowText
        && wrote.0 > allowed
        && wrote.0 > RUN_BUDGET.floor.0;
    let past_declaration = growth == Growth::AtMostInput && wrote.0 > fed.0;
    if past_budget || past_declaration {
        return Err(GrowthError::OverBudget { fed, wrote });
    }
    Ok(())
}

/// Where `making` falls against the run's worst making so far: `Greater` when it becomes the worst, the one that wrote the
/// most rows per row fed of those that wrote at least the budget's floor.
pub(crate) fn against_worst(making: Making, worst: Option<Making>) -> Ordering {
    if making.wrote.0 < RUN_BUDGET.floor.0 {
        return Ordering::Less;
    }
    worst.map_or(Ordering::Greater, |worst| fan_out(making, worst))
}

/// The rows a making was fed, every input summed.
pub(crate) fn fed_rows(fed: &[RowCount]) -> RowCount {
    RowCount(
        fed.iter()
            .fold(0_usize, |sum, rows| sum.saturating_add(rows.0)),
    )
}

/// How `making`'s rows written per row fed compare with `than`'s, a making fed no row counted as fed one.
fn fan_out(making: Making, than: Making) -> Ordering {
    let rows = |count: RowCount| u128::try_from(count.0).unwrap_or(u128::MAX);
    rows(making.wrote)
        .saturating_mul(rows(than.fed).max(1))
        .cmp(&rows(than.wrote).saturating_mul(rows(making.fed).max(1)))
}
