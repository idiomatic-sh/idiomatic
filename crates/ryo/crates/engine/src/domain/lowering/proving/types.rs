//! The types of the `proving` part: the unit a fault is found in, and what the fault is.

use crate::domain::lowering::types::PastWidth;
use crate::domain::steps::types::{DefinitionId, MadeTableId};

/// A unit of a lowered pack.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProvedUnit {
    /// A rule's own program, by the rule's place among the pack's rules.
    Rule(crate::domain::lowering::nodes::types::AcceptedPlace),
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

/// Where each unit of a pack was lowered from: what a fault of the proof names its rule by.
#[derive(Clone, Debug, Default)]
pub(crate) struct UnitHomes {
    /// The home of each definition, by number.
    pub(crate) definitions: Vec<crate::domain::lowering::nodes::types::NodeHome>,
    /// The home of each made table, by number.
    pub(crate) tables: Vec<crate::domain::lowering::nodes::types::NodeHome>,
}

/// Where a program stands.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum ProgramStanding {
    /// A rule's branch: its first step may be file-sized, and may read a definition's rows whole.
    Branch,
    /// Under a candidate or a key: an inline body, a call-form pattern, a slot of a `Fork`.
    Keyed,
}

/// What a step holds: the programs it runs in place, and the operands of its widest list.
#[derive(Clone, Debug)]
pub(crate) struct HeldPrograms<'a> {
    /// The inline programs.
    pub(crate) programs: Vec<&'a crate::domain::steps::types::StepProgram>,
    /// The operands of the widest list.
    pub(crate) operands: ryo_relations::domain::relations::types::Count,
}

/// Whether a step reads a definition's rows whole.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum WholeRead {
    /// It does: only `EachRow` may, first in a rule's branch.
    Whole,
    /// It does not.
    Keyed,
}
