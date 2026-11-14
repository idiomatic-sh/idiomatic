//! The program the engine builds node by node and the planner reads: every rule's nodes, merged where equal, each rule's
//! emitted node and the consumers that are not node readers.

use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::physical_plan::types::{
    Emitted, LogicalNode, PlanInput, PlanNodeId, RuleOrdinal,
};
use ryo_values::domain::descriptions::types::KindVocabulary;

/// A logical program over one kind vocabulary.
///
/// # Examples
///
/// ```
/// use ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram;
/// use ryo_values::domain::descriptions::types::KindVocabulary;
/// fn empty(vocabulary: &KindVocabulary) -> LogicalProgram<'_> {
///     LogicalProgram::new(vocabulary)
/// }
/// ```
#[derive(Clone, Debug)]
pub struct LogicalProgram<'p> {
    /// The kinds the program's kind tests name.
    #[expect(
        dead_code,
        reason = "read by the root kinds of the program, which the engine derives with its kind sets when the file pass lands (F1 commit 6)"
    )]
    vocabulary: &'p KindVocabulary,
    /// The nodes, each reading only earlier ones.
    nodes: Vec<LogicalNode<'p>>,
    /// Each rule's emitted node, in rule order.
    emits: Vec<Emitted>,
    /// The consumers that read no node: frontier captures, observation roots, publication.
    holds: Vec<PlanNodeId>,
}

impl<'p> LogicalProgram<'p> {
    /// A program with no node over `vocabulary`.
    #[must_use]
    pub fn new(vocabulary: &'p KindVocabulary) -> Self {
        Self {
            vocabulary,
            nodes: Vec::new(),
            emits: Vec::new(),
            holds: Vec::new(),
        }
    }

    /// Appends a node whose `PlanInput::Node` inputs are all earlier ids; answers its id. A node that reads a later one is
    /// refused by `plan`, not here.
    pub fn node(&mut self, node: LogicalNode<'p>) -> PlanNodeId {
        let id = PlanNodeId(u32::try_from(self.nodes.len()).unwrap_or(u32::MAX));
        self.nodes.push(node);
        id
    }

    /// `rule`'s emitted node; rules in rule order.
    pub fn emit(&mut self, rule: RuleOrdinal, node: PlanNodeId) {
        self.emits.push(Emitted { rule, node });
    }

    /// One more consumer that is not a node reader: frontier capture, observation root, publication.
    pub fn hold(&mut self, node: PlanNodeId) {
        self.holds.push(node);
    }

    /// The nodes, in node order.
    pub(crate) fn nodes(&self) -> &[LogicalNode<'p>] {
        &self.nodes
    }

    /// Each rule's emitted node, in rule order.
    pub(crate) fn emitted(&self) -> impl Iterator<Item = PlanNodeId> + '_ {
        self.emits.iter().map(|emitted| emitted.node)
    }

    /// The consumers that read no node.
    pub(crate) fn held(&self) -> &[PlanNodeId] {
        &self.holds
    }

    /// Checks the program's shape: every node reads only earlier nodes, and every emitted or held node is one of the program's.
    ///
    /// # Errors
    ///
    /// `WrongOperation` naming no node: a program of another shape is not a program.
    pub(crate) fn check_shape(&self) -> Result<(), PortableOperatorFault> {
        let count = PlanNodeId(u32::try_from(self.nodes.len()).unwrap_or(u32::MAX));
        let earlier = |at: PlanNodeId, input: &PlanInput| match input {
            PlanInput::Node(read) => *read < at,
            PlanInput::Relation(_) | PlanInput::Bound(_) | PlanInput::Observation(_) => true,
        };
        let shaped = self.emits.iter().all(|emitted| emitted.node < count)
            && self.holds.iter().all(|held| *held < count)
            && self
                .nodes
                .iter()
                .zip((0..).map(PlanNodeId))
                .all(|(node, at)| node.inputs.0.iter().all(|input| earlier(at, input)));
        if shaped {
            Ok(())
        } else {
            Err(PortableOperatorFault::WrongOperation)
        }
    }
}
