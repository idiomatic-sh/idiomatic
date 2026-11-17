//! The program the file pass runs: the physical plan the operators made of a request's logical program, each rule's nodes,
//! whether each rule is served, each node's consumers and the root kinds.

use crate::domain::executable_program::types::{
    Consumers, ObservationReach, RuleNodes, RuleScopes,
};
use crate::domain::operating::errors::OperatingError;
use ryo_operators::domain::physical_plan::types::PlanNodeId;
use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::LiteralText;

/// A request's program, planned. Read through the concept's service: `roots`, `rule`, `consumers`, `physical`.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::service::{assemble, logical, physical};
/// use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
/// use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
/// let mut inputs = ProgramInputs::default();
/// let build = logical(&[], &mut inputs).unwrap();
/// let program = assemble(build, PhysicalPlan::default());
/// let _plan = physical(&program);
/// ```
#[derive(Clone, Debug)]
pub struct ExecutableProgram {
    /// The operators' plan of the logical program.
    pub(crate) plan: PhysicalPlan,
    /// Each rule's nodes, in rule order.
    pub(crate) rules: Vec<RuleNodes>,
    /// Each node's consumers, by node.
    pub(crate) consumers: Vec<Consumers>,
    /// Each rule's engine uses, in rule order: the node it emits, then each node it holds.
    pub(crate) engine: Vec<Vec<PlanNodeId>>,
    /// The nodes that start from the tree nodes of a kind, by the text the plans write for the kind.
    pub(crate) roots: hashbrown::HashMap<LiteralText, Vec<PlanNodeId>>,
    /// The columns each node's rows answer with as a root, by node: its schema, or the empty layout of a refusal deferred to
    /// its rows.
    pub(crate) layouts: Vec<Result<Vec<CellKind>, OperatingError>>,
    /// The inputs each node reads, by node, in the order it reads them.
    pub(crate) reads: Vec<Vec<ryo_operators::domain::physical_plan::types::PlanInput>>,
    /// The layout of each node whose refusal is deferred to its rows, by node; none for any other.
    pub(crate) deferred: Vec<Option<Vec<CellKind>>>,
    /// Where each rule's emitted step and named outputs run, in rule order.
    pub(crate) scopes: Vec<RuleScopes>,
    /// Whether each node's rows answer the phase or one occurrence, by node.
    pub(crate) reach: Vec<ObservationReach>,
}
