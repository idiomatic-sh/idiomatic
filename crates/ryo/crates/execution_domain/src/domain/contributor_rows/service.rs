//! What the rows of an answer keep of the operands whose absence of rows an operator trusts.
use crate::domain::completion::types::CoverageState;
use crate::domain::operator::contract::types::AbsenceMask;
use crate::domain::output_coverage::types::InputCoverage;

/// MissingRows named populations obey the same absence semantics as physical reads.
/// Positive operators retain admitted rows; absence-sensitive operators can
/// trust a keyed answer only outside the affected physical partitions.
pub fn filter_contributor_rows(
    operation: &ryo_values::domain::plans::types::Operation,
    absence: AbsenceMask,
    inputs: &[InputCoverage<'_>],
    rows: &mut ryo_values::domain::rows::types::Table,
) {
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
