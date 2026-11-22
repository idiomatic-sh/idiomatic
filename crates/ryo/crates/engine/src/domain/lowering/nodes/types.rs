//! The types of the `nodes` part: a rule's steps as the lowering reads them, and the pack's store of merged plan nodes.

use crate::domain::lowering::types::SelectedRule;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::{ColumnIndex, InputNumber, StepId};
use ryo_values::domain::query_plans::types::{NodeDigest, NodeDigests};
use ryo_values::domain::rule_requests::types::SelectedRuleOrdinal;
use rustc_hash::{FxHashMap as HashMap, FxHashSet as HashSet};
use std::collections::BTreeSet;

/// What each column of a step's rows holds, in column order.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct StepShape(pub(crate) Vec<CellKind>);

/// One accepted rule as the lowering reads it: its plan, each step's digest, and the shape of every step its emit reaches.
#[derive(Clone, Debug)]
pub(crate) struct AdmittedRule<'a> {
    /// The rule.
    pub(crate) rule: SelectedRule<'a>,
    /// The digest of every step, in step order.
    pub(crate) digests: NodeDigests,
    /// The shape of each step the emit reaches, by step; none for a step it does not reach.
    pub(crate) shapes: Vec<Option<StepShape>>,
    /// What the host declares beside the plans: the rows of a `Hosted` step and the producers of a bound observation.
    pub(crate) given: crate::domain::lowering::types::LoadGiven<'a>,
    /// Whose rows each shaped step holds: the file's, or the same rows for every file.
    pub(crate) locals: Vec<Option<crate::domain::lowering::observing::types::Local>>,
    /// The steps of this rule a bound observation of another rule reads: each is one definition of the pack.
    pub(crate) observed: Vec<StepId>,
    /// The bound observations this rule reads, each with the producers the load accepted.
    pub(crate) bound: Vec<crate::domain::lowering::observing::types::ResolvedBinding>,
}

/// Who reads a merged plan node: a step of some rule by one of its inputs, or a rule's emit.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(crate) enum NodeReader {
    /// A step, named by its own digest, through the input of this number.
    Step(NodeDigest, InputNumber),
    /// The emit of a rule.
    Emit(SelectedRuleOrdinal),
    /// A step of a rule whose digest the rule holds at several places, by its place and the input: each place is a reader
    /// of its own, since each is flattened (`tables::arms`) and takes its input's branches.
    Placed(SelectedRuleOrdinal, StepId, InputNumber),
}

/// Where a merged plan node is lowered from: the first accepted rule that holds it, and its place there.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct NodeHome {
    /// The rule.
    pub(crate) rule: AcceptedPlace,
    /// The step, in that rule's plan.
    pub(crate) step: StepId,
}

/// The columns of a merged plan node its readers across the pack read.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ReadColumns {
    /// Every reader is a projection: the columns they keep, together.
    Kept(BTreeSet<ColumnIndex>),
    /// Some reader reads the whole row: an operation that is no projection, or a rule's emit.
    Whole,
}

/// One merged plan node of the pack.
#[derive(Clone, Debug)]
pub(crate) struct StoredNode {
    /// Where it is lowered from.
    pub(crate) home: NodeHome,
    /// Its distinct readers across the pack.
    pub(crate) readers: HashSet<NodeReader>,
    /// The columns of it they read.
    pub(crate) read: ReadColumns,
}

/// The pack's one store of merged plan nodes, by digest.
#[derive(Clone, Debug, Default)]
pub(crate) struct NodeStore(pub(crate) HashMap<NodeDigest, StoredNode>);

/// An accepted rule, by its place among the accepted rules.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub(crate) struct AcceptedPlace(pub(crate) usize);

/// Whether a rule's emit reaches a step.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum StepReach {
    /// It does.
    Reached,
    /// It does not: the step is never lowered.
    Unreached,
}
