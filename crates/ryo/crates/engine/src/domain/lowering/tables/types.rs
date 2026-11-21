//! The types of the `tables` part: what the flattening of one rule holds, and how a step asks for an input.

use crate::domain::lowering::definitions::types::Flattened;
use crate::domain::lowering::flattening::types::RowColumn;
use crate::domain::lowering::nodes::types::{AcceptedPlace, AdmittedRule, NodeStore};
use ryo_relations::domain::relations::types::Count;
use ryo_values::domain::descriptions::types::KindVocabulary;

/// The flattening of one rule: the pack's rules and store, the rule's place, and each of its steps flattened so far. A step's
/// flattening is taken by its one reader, or becomes a definition.
#[derive(Debug)]
pub(crate) struct RuleFlattening<'a> {
    /// The accepted rules.
    pub(crate) rules: &'a [AdmittedRule<'a>],
    /// The pack's merged plan nodes.
    pub(crate) store: &'a NodeStore,
    /// The language's kinds.
    pub(crate) kinds: &'a KindVocabulary,
    /// The rule being flattened, by its place among the accepted rules.
    pub(crate) place: AcceptedPlace,
    /// Each step's flattening, by step, until it is read.
    pub(crate) flat: Vec<Option<Flattened>>,
}

/// How a step asks for an input.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum InputWant {
    /// The input's own branches, unless it has several readers or is past a bound.
    Inline,
    /// One use of the input's definition, whatever its size: the right side of an absence, the rows of a fill.
    Defined,
}

/// How the lowering walks a plan's axis.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TreeAxis {
    /// Each element along a tree axis.
    Along(crate::domain::steps::types::AxisDirection),
    /// The nearest ancestor the axis's domain holds.
    Nearest,
}

/// The size of an input as its reader would hold it: its branches, and the atoms of its largest.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct InputSize {
    /// The branches.
    pub(crate) branches: Count,
    /// The atoms of the largest branch.
    pub(crate) atoms: Count,
}

/// How each side of a join is asked for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct JoinWants {
    /// The left side.
    pub(crate) left: InputWant,
    /// The right side.
    pub(crate) right: InputWant,
}

/// One key of a join: a column of its left side equal to a column of its right.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct KeyPair {
    /// The left column.
    pub(crate) left: RowColumn,
    /// The right column.
    pub(crate) right: RowColumn,
}

/// What a made table is of: the merged plan node it is the rows of, where that node was lowered from, and what a file must
/// hold for it to have a row.
#[derive(Debug)]
pub(crate) struct TableOf {
    /// The node.
    pub(crate) node: ryo_values::domain::query_plans::types::NodeDigest,
    /// Where it was lowered from.
    pub(crate) home: crate::domain::lowering::nodes::types::NodeHome,
    /// Its emptiness.
    pub(crate) emptiness: crate::domain::lowered::types::Emptiness,
}
