//! The types of the `proving` part: the unit a fault is found in, and what the fault is.

use crate::domain::lowering::types::PastWidth;
use crate::domain::steps::types::{DefinitionId, MadeTableId};

/// A unit of a lowered pack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProvedUnit {
    /// A rule's own program, by the rule's place among the pack's rules.
    Rule(usize),
    /// A definition's call-form patterns.
    Definition(DefinitionId),
    /// A made table's fill.
    Table(MadeTableId),
}

/// What is wrong with a unit.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProofFault {
    /// A file-sized step or stage stands where none may, or a product stage stands in a table of no keyless join, or a
    /// definition's rows are read whole where they may not be: the lowering's own fault.
    Misplaced,
    /// A count of the unit does not fit the width the code gives it.
    Past(PastWidth),
}

/// A unit that does not hold the invariant.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Unproved {
    /// The unit.
    pub(crate) unit: ProvedUnit,
    /// What is wrong with it.
    pub(crate) fault: ProofFault,
}

/// Whether a made table's own plan states a join with no key within the file: only such a table holds a product stage.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProductStated {
    /// It does.
    Stated,
    /// It does not: every join of it is keyed.
    Keyed,
}
