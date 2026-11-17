//! Set policy controls membership, distinctness, laziness and emission order.
use crate::domain::set_rows::types::SetKind;
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// The one lowering of each set meaning: a union is `distinct` over both inputs; an intersection and a difference a semi and
/// an anti probe of the whole rows, `distinct`; an antijoin an anti probe of its keys, which keeps every left row of no right
/// rows unread.
pub(crate) fn lower(
    kind: SetKind,
    left_on: &[ColumnIndex],
    right_on: &[ColumnIndex],
) -> crate::domain::set_rows::types::SetLowering {
    use crate::domain::equality_join::types::{
        EmptyRight, Existence, ProbeKey, ProbeMode, ProbeParts,
    };
    use crate::domain::set_rows::types::SetLowering;
    use ryo_values::domain::plan_parts::types::RepeatedRows;
    let probe = |mode, key, repeated, empty_right| {
        SetLowering::Probe(ProbeParts {
            mode,
            key,
            filter: crate::domain::select::service::lower(&[], &[], &[]),
            output: Vec::new(),
            existence: Existence::Every,
            repeated,
            empty_right,
        })
    };
    match kind {
        SetKind::Union => SetLowering::Distinct,
        SetKind::Intersection => probe(
            ProbeMode::Semi,
            ProbeKey::Whole,
            RepeatedRows::Dropped,
            EmptyRight::KeyRead,
        ),
        SetKind::Difference => probe(
            ProbeMode::Anti,
            ProbeKey::Whole,
            RepeatedRows::Dropped,
            EmptyRight::KeyRead,
        ),
        SetKind::Antijoin => probe(
            ProbeMode::Anti,
            ProbeKey::Columns {
                left: left_on.to_vec(),
                right: right_on.to_vec(),
            },
            RepeatedRows::Kept,
            EmptyRight::EveryLeftRow,
        ),
    }
}
