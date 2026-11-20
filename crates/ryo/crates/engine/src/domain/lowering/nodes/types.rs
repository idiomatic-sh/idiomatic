//! The types of the `nodes` part: a rule's steps as the lowering reads them, and the pack's store of merged plan nodes.

use crate::domain::lowering::types::SelectedRule;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{InputNumber, StepId};
use ryo_values::domain::query_plans::types::{NodeDigest, NodeDigests};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use std::collections::{HashMap, HashSet};

/// What each column of a step's rows holds, in column order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct StepShape(pub(crate) Vec<CellKind>);

/// One accepted rule as the lowering reads it: its plan, each step's digest, and the shape of every step its emit reaches.
#[derive(Clone, Debug)]
pub(crate) struct RuleNodes<'a> {
    /// The rule.
    pub(crate) rule: SelectedRule<'a>,
    /// The digest of every step, in step order.
    pub(crate) digests: NodeDigests,
    /// The shape of each step the emit reaches, by step; none for a step it does not reach.
    pub(crate) shapes: Vec<Option<StepShape>>,
}

/// Who reads a merged plan node: a step of some rule by one of its inputs, or a rule's emit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum NodeReader {
    /// A step, named by its own digest, through the input of this number.
    Step(NodeDigest, InputNumber),
    /// The emit of a rule.
    Emit(SelectedRuleOrdinal),
}

/// Where a merged plan node is lowered from: the first accepted rule that holds it, and its place there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NodeHome {
    /// The rule, by its place among the accepted rules.
    pub(crate) rule: usize,
    /// The step, in that rule's plan.
    pub(crate) step: StepId,
}

/// One merged plan node of the pack.
#[derive(Clone, Debug)]
pub(crate) struct StoredNode {
    /// Where it is lowered from.
    pub(crate) home: NodeHome,
    /// Its distinct readers across the pack.
    pub(crate) readers: HashSet<NodeReader>,
}

/// The pack's one store of merged plan nodes, by digest.
#[derive(Clone, Debug, Default)]
pub(crate) struct NodeStore(pub(crate) HashMap<NodeDigest, StoredNode>);
