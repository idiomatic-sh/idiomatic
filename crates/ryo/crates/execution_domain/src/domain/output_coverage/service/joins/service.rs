//! Where the rows missing from an operand of a join would lie: proven from the lineage of both operands, or carried from the
//! missing operand's own lineage through the keys.
use crate::domain::output_coverage::types::{InputCoverage, OperandOrdinal, Proof};
use ryo_values::domain::lineage::service::{anchor_at, anchor_of, anchor_through, pairing};
use ryo_values::domain::lineage::types::{ColumnAnchor, KeyPairing};
use ryo_values::domain::plan_parts::types::{ColumnIndex, JoinColumn};

/// Whether the lineage of the two operands of a join proves where the rows missing from either would lie: a key
/// pairs a column of each operand that lies in its row's file, so the two rows a missing one would have been combined with come
/// from its file, and every file or node column of the answer lies in its row's file, so it lies in the missing file too. The
/// right keys and the answer's right columns are read through the view.
pub(crate) fn proven_by_lineage(
    inputs: &[InputCoverage<'_>],
    left_on: &[ColumnIndex],
    right_on: &[ColumnIndex],
    output: &[JoinColumn],
    view: &[ColumnIndex],
    output_kinds: &[ryo_relations::domain::relations::types::CellKind],
) -> Proof {
    use ryo_values::domain::plan_parts::types::Side;
    let lineage_of = |at: OperandOrdinal| {
        inputs
            .iter()
            .find(|each| each.ordinal == at)
            .map(|each| &each.lineage)
    };
    let (Some(left), Some(right)) = (lineage_of(OperandOrdinal(0)), lineage_of(OperandOrdinal(1)))
    else {
        return Proof::Unproven;
    };
    if pairing(left, left_on, right, right_on, view) == KeyPairing::Unproven {
        return Proof::Unproven;
    }
    let anchored = output.iter().zip(output_kinds).all(|(column, kind)| {
        anchor_of(*kind) != ColumnAnchor::InFile
            || match column.side {
                Side::Left => anchor_at(left, column.column),
                Side::Right => anchor_through(right, view, column.column),
            } == ColumnAnchor::InFile
    });
    if anchored && output.len() == output_kinds.len() {
        Proof::Proven
    } else {
        Proof::Unproven
    }
}

/// Whether every file or node column of the answer lies in the file the rows missing from `missing` come from, by that operand's
/// lineage alone: a column of the missing operand lies there when its lineage says so, and a column of the other operand when it
/// is a key paired with a key of the missing operand that lies in its row's file and holds the same kind, as the two hold the same
/// file or node. The other operand's own lineage is not needed: the key equality carries the missing file into its columns. The
/// right keys and the answer's right columns are read through the view.
pub(crate) fn transported(
    missing: &InputCoverage<'_>,
    left_on: &[ColumnIndex],
    right_on: &[ColumnIndex],
    output: &[JoinColumn],
    view: &[ColumnIndex],
    output_kinds: &[ryo_relations::domain::relations::types::CellKind],
) -> Proof {
    use ryo_values::domain::plan_parts::types::Side;
    let own_side = if missing.ordinal.0 == 0 {
        Side::Left
    } else {
        Side::Right
    };
    // The missing operand's own column a join names: through the view on the right.
    let own = |column: ColumnIndex| {
        if own_side == Side::Right {
            ryo_values::domain::plan_parts::service::view_source(view, column)
        } else {
            Some(column)
        }
    };
    let in_file = |column: ColumnIndex| anchor_at(&missing.lineage, column) == ColumnAnchor::InFile;
    let (missing_on, other_on) = if own_side == Side::Left {
        (left_on, right_on)
    } else {
        (right_on, left_on)
    };
    let anchored = output.iter().zip(output_kinds).all(|(column, kind)| {
        if anchor_of(*kind) != ColumnAnchor::InFile {
            return true;
        }
        if column.side == own_side {
            return own(column.column).is_some_and(in_file);
        }
        other_on.iter().zip(missing_on).any(|(other, key)| {
            *other == column.column
                && own(*key).is_some_and(|key| {
                    in_file(key)
                        && usize::try_from(key.0)
                            .is_ok_and(|at| missing.kinds.get(at) == Some(kind))
                })
        })
    });
    if anchored && output.len() == output_kinds.len() {
        Proof::Proven
    } else {
        Proof::Unproven
    }
}
