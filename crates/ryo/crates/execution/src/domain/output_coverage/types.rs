//! The types of the `output_coverage` concept: plain data, public fields and derives, with no constructor and no inherent impl.

use crate::domain::completion::types::CoverageState;

/// The place of an operand among the operands of an operator.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct OperandOrdinal(pub usize);

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
pub(crate) struct InputCoverage<'a> {
    pub ordinal: OperandOrdinal,
    pub kinds: &'a [ryo_relations::domain::relations::types::CellKind],
    pub coverage: CoverageState,
}
