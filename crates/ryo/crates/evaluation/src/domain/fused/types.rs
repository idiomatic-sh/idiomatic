//! The types of the `fused` concept. Plain data, public fields and derives.

use ryo_relations::domain::relations::types::KindBits;
use ryo_values::domain::plans::types::Operation;

/// A step of the fused program, by its place: the program's steps are in an order where each reads only those before it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct NodeId(pub usize);

/// Which kinds of element a root select may keep.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Admits {
    /// Any kind: its comparisons do not name one.
    Every,
    /// Only these kinds.
    Only(Vec<KindBits>),
}

/// A select over the parser's tree, served by the one walk.
#[derive(Clone, Debug, PartialEq)]
pub struct Root {
    /// Its step.
    pub node: NodeId,
    /// The kinds it may keep.
    pub admits: Admits,
}

/// The rules of a request fused into one program.
#[derive(Clone, Debug, PartialEq)]
pub struct Fused {
    /// Every distinct step, each with its inputs read as `Input::Step(StepId(node))` of this program.
    pub nodes: Vec<Operation>,
    /// The step each rule emits, by the rule's place.
    pub emits: Vec<NodeId>,
    /// The selects over the parser's tree.
    pub roots: Vec<Root>,
    /// The roots each kind of element feeds, by the kind's number.
    pub by_kind: Vec<Vec<usize>>,
    /// The roots every element feeds.
    pub every: Vec<usize>,
}
