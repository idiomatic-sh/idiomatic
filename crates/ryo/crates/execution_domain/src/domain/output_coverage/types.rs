//! The types of the `output_coverage` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::completion::types::CoverageState;

/// The place of an operand among the operands of an operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct OperandOrdinal(pub usize);

/// What an operand of an operator reads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum OperandReads {
    /// A relation ryo owns: its file and node columns lie in the file its row is partitioned by.
    Relation,
    /// The rows of another step: its columns anchor nothing.
    Step,
}

/// One actually admitted operand's schema and physical completeness. MissingRows
/// operands are intentionally absent when a lazy demand gate did not read them.
pub struct InputCoverage<'a> {
    pub ordinal: OperandOrdinal,
    pub kinds: &'a [ryo_relations::domain::relations::types::CellKind],
    pub coverage: CoverageState,
    /// Where the operand's rows come from: which of its columns lie in the file a row comes from.
    pub lineage: crate::domain::lineage::types::Lineage,
}

/// Whether the lineage of a join's operands proves where the rows missing from one of them would lie.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Proof {
    /// Every file or node column of the answer lies in the file the missing rows come from.
    Proven,
    /// The lineage does not show where the missing rows would lie.
    Unproven,
}
