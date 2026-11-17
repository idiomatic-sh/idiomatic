//! The types of the `activation` concept: plain data, public fields and derives, with no constructor and no inherent impl.
use crate::domain::operator::contract::presence::types::{InputMask, PresenceMask};
use ryo_operators::domain::physical_plan::types::PlanNodeId;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::LiteralText;

/// What each node of one planned program needs to be active, by node id, and the triggers a file is asked for.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::activation::types::ActivationPlan;
/// assert_eq!(ActivationPlan::default(), ActivationPlan::default());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ActivationPlan {
    /// Each node's requirement, by node id: nodes come after the nodes they read.
    pub nodes: Box<[NodeRequirement]>,
    /// The root kinds a file is asked whether it holds, by kind slot.
    pub kinds: Box<[LiteralText]>,
    /// The per-file fact relations a file is asked whether it has a row of, by fact slot.
    pub facts: Box<[Relation]>,
}

/// What one node needs to be active in a file.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::activation::types::NodeRequirement;
/// assert_eq!(NodeRequirement::Live, NodeRequirement::Live);
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum NodeRequirement {
    /// It may have rows and row errors in any file: a step after the files, a step reading a frontier or a bound observation,
    /// a hosted step, or a step whose schema is refused or whose refusal is deferred.
    Live,
    /// A tree select whose infallible prefix names these kinds: it has rows only where the file holds one of them, and a row
    /// error only where it has a row and a comparison after the prefix can fail (`fallible`).
    Kinds {
        /// The kinds its prefix names, by kind slot.
        slots: Box<[KindSlot]>,
        /// The inputs whose rows can make it fail, as its contract declares (`failure`).
        fallible: InputMask,
    },
    /// An operation over its inputs, as its contract declares.
    Inputs {
        /// The inputs that must have a row for it to answer one (`semantics(kind).presence`).
        rows: PresenceMask,
        /// The inputs whose rows can make it fail (`failure`, `UnlessLocated` read as its first input).
        fallible: InputMask,
        /// Its inputs, in operand order.
        from: Box<[ActivationInput]>,
    },
}

/// One input of a node, as activation reads it.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::activation::types::ActivationInput;
/// assert_eq!(ActivationInput::Tree, ActivationInput::Tree);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActivationInput {
    /// An earlier node.
    Node(PlanNodeId),
    /// A per-file fact relation, by its slot.
    Fact(FactSlot),
    /// A tree relation read whole: the file always has tree rows, and reading them cannot fail.
    Tree,
}

/// A root kind's place among a plan's kinds.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::activation::types::KindSlot;
/// assert_eq!(KindSlot(0).0, 0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct KindSlot(pub usize);

/// A fact relation's place among a plan's facts.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::activation::types::FactSlot;
/// assert_eq!(FactSlot(0).0, 0);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct FactSlot(pub usize);

/// Whether a file holds a trigger.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::activation::types::TriggerPresence;
/// assert_eq!(TriggerPresence::default(), TriggerPresence::Present);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TriggerPresence {
    /// It does, or the file cannot say (a refused or partial read).
    #[default]
    Present,
    /// It does not.
    Absent,
}

/// What one file holds of a plan's root kinds, by kind slot. A file whose reads are partial holds every kind.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::activation::types::Triggers;
/// assert!(Triggers::default().kinds.is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Triggers {
    /// Each root kind's presence, by kind slot.
    pub kinds: Vec<TriggerPresence>,
}

/// What a file's one read of a fact relation answered, as activation reads it.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::activation::types::FactAnswer;
/// assert_ne!(FactAnswer::Rows, FactAnswer::NoRow);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FactAnswer {
    /// Rows, or a file that cannot say it has none (a partial read).
    Rows,
    /// No row.
    NoRow,
    /// The read was refused: the reader runs and meets the refusal.
    Refused,
}

/// The file's answers activation reads, handed in by the caller: what one read of a fact relation answered. Each relation is
/// read once in the file, so its answer here is the read its readers get.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::activation::types::{ActivationLeaves, FactAnswer};
/// use ryo_relations::domain::relations::types::Relation;
/// struct Empty;
/// impl ActivationLeaves for Empty {
///     fn rows(&mut self, _relation: Relation) -> FactAnswer {
///         FactAnswer::NoRow
///     }
/// }
/// assert_eq!(Empty.rows(Relation::Function), FactAnswer::NoRow);
/// ```
pub trait ActivationLeaves {
    /// What the file's one read of `relation` answered.
    fn rows(&mut self, relation: Relation) -> FactAnswer;
}

/// What deciding a node's state came to.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::activation::types::NodeDecision;
/// assert_ne!(NodeDecision::Refused, NodeDecision::Decided(Default::default()));
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeDecision {
    /// Its state, from its inputs'.
    Decided(NodeState),
    /// A read on the way was refused: the node runs and meets the refusal.
    Refused,
}

/// One operation's requirement over its inputs, as a decision reads it.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::activation::types::NodeInputs;
/// use ryo_execution_domain::domain::operator::contract::presence::types::{InputMask, PresenceMask};
/// use ryo_operators::domain::physical_plan::types::PlanNodeId;
/// let inputs = NodeInputs { node: PlanNodeId(0), rows: PresenceMask::Always, fallible: InputMask(0), from: &[] };
/// assert!(inputs.from.is_empty());
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct NodeInputs<'p> {
    /// The node.
    pub node: PlanNodeId,
    /// The inputs that must have a row for it to answer one.
    pub rows: PresenceMask,
    /// The inputs whose rows can make it fail.
    pub fallible: InputMask,
    /// Its inputs, in operand order.
    pub from: &'p [ActivationInput],
}

/// Whether a node runs in the open file.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::activation::types::NodeActivity;
/// assert_ne!(NodeActivity::Active, NodeActivity::Dormant);
/// ```
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NodeActivity {
    /// It may have a row or a row error: it runs.
    Active,
    /// It has neither: it answers the empty table of its layout without reading an input.
    Dormant,
}

/// What a node can hold in one file: rows, and a row error of its own or carried from upstream.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::activation::types::{NodeState, TriggerPresence};
/// assert_eq!(NodeState::default().faults, TriggerPresence::Present);
/// ```
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct NodeState {
    /// Whether it can have a row.
    pub rows: TriggerPresence,
    /// Whether it can raise or carry a row error.
    pub faults: TriggerPresence,
}

/// Each node's state in the open file, by node id, once it was composed: cleared per file, keeping its room.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::activation::types::Activities;
/// assert!(Activities::default().0.is_empty());
/// ```
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Activities(pub Vec<Option<NodeState>>);
