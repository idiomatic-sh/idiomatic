//! The types of the `tables` part: what the flattening of one rule holds, and how a step asks for an input.

use crate::domain::lowering::definitions::types::Flattened;
use crate::domain::lowering::nodes::types::{AcceptedPlace, AdmittedRule, NodeStore};
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
