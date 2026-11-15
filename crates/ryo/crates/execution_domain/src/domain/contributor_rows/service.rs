//! What the rows of an answer keep of the operands whose absence of rows an operator trusts.
use crate::domain::completion::types::CoverageState;
use crate::domain::operator::contract::types::AbsenceMask;
use crate::domain::output_coverage::types::InputCoverage;

/// MissingRows named populations obey the same absence semantics as physical reads.
/// Positive operators retain admitted rows; absence-sensitive operators can
/// trust a keyed answer only outside the affected physical partitions, and outside the files its rule skipped (`skipped`,
/// sorted, once each), which lack input whatever its operands read. After the files the project pass calls it once per antijoin
/// or difference whose operand is partial or whose rule skipped a file, on the step's rows where the step stands.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::contributor_rows::service::filter_contributor_rows;
/// use ryo_execution_domain::domain::operator::contract::types::AbsenceMask;
/// use ryo_values::domain::plan_parts::types::{Input, StepId};
/// use ryo_values::domain::plans::types::Operation;
/// use ryo_values::domain::rows::types::Table;
/// let difference = Operation::Difference { left: Input::Step(StepId(0)), right: Input::Step(StepId(1)) };
/// let mut rows = Table::default();
/// filter_contributor_rows(&difference, AbsenceMask(0b10), &[], &[], &mut rows);
/// assert!(rows.cells.is_empty());
/// ```
pub fn filter_contributor_rows(
    operation: &ryo_values::domain::plans::types::Operation,
    absence: AbsenceMask,
    inputs: &[InputCoverage<'_>],
    skipped: &[ryo_relations::domain::relations::types::FileId],
    rows: &mut ryo_values::domain::rows::types::Table,
) {
    if !skipped.is_empty() {
        let kinds = rows.kinds.clone();
        crate::domain::coverage::service::drop_inapplicable(operation, &kinds, rows, skipped);
    }
    for input in inputs {
        if !u32::try_from(input.ordinal.0).is_ok_and(|ordinal| ordinal < u8::BITS)
            || absence.0 & (1 << input.ordinal.0) == 0
        {
            continue;
        }
        match &input.coverage {
            CoverageState::Complete => {}
            CoverageState::Partial { files, .. } => {
                let mut affected = files.clone();
                affected.sort_unstable();
                affected.dedup();
                crate::domain::coverage::service::drop_uncertain(
                    operation,
                    input.kinds,
                    rows,
                    &affected,
                );
            }
            CoverageState::Inapplicable { files } => {
                let mut inapplicable = files.clone();
                inapplicable.sort_unstable();
                inapplicable.dedup();
                crate::domain::coverage::service::drop_inapplicable(
                    operation,
                    input.kinds,
                    rows,
                    &inapplicable,
                );
            }
        }
    }
}
