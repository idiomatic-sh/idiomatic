//! The program the file pass runs: the physical plan the operators made of a request's logical program, each rule's nodes,
//! whether each rule is served, each node's consumers and the root kinds.

use crate::domain::executable_program::types::{Consumers, RuleNodes, Serving};
use crate::domain::operating::errors::OperatingError;
use ryo_operators::domain::physical_plan::types::PlanNodeId;
use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::LiteralText;

/// A request's program, planned. Read through the concept's service: `serving`, `roots`, `rule`, `consumers`, `physical`.
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
/// assert!(physical(&program).unserved().is_empty());
/// ```
#[derive(Clone, Debug)]
pub struct ExecutableProgram {
    /// The operators' plan of the logical program.
    pub(crate) plan: PhysicalPlan,
    /// Each rule's nodes, in rule order.
    pub(crate) rules: Vec<RuleNodes>,
    /// Whether each rule is served.
    pub(crate) serving: Vec<Serving>,
    /// Each node's consumers, by node.
    pub(crate) consumers: Vec<Consumers>,
    /// The nodes that start from the tree nodes of a kind, by the text the plans write for the kind.
    pub(crate) roots: hashbrown::HashMap<LiteralText, Vec<PlanNodeId>>,
    /// Each node's output columns, or the schema fault of it or of a node it reads, by node.
    pub(crate) schemas: Vec<Result<Vec<CellKind>, OperatingError>>,
}
