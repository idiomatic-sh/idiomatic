//! The logical program of a request, with each rule's nodes, whether each rule is served, each node's consumers and the root
//! kinds, until the operators' physical plan completes it.

use crate::domain::executable_program::types::{Consumers, RuleNodes, Serving};
use crate::domain::operating::errors::OperatingError;
use ryo_operators::domain::physical_plan::types::PlanNodeId;
use ryo_operators::domain::physical_plan::types::logical_program::value::LogicalProgram;
use ryo_relations::domain::relations::types::CellKind;
use ryo_values::domain::plan_parts::types::LiteralText;

/// A request's logical program and what the engine derived while building it.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::service::logical;
/// use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
/// let mut inputs = ProgramInputs::default();
/// let build = logical(&[], &mut inputs).unwrap();
/// assert!(format!("{:?}", build.program()).contains("nodes: []"));
/// ```
#[derive(Clone, Debug)]
pub struct LogicalBuild<'p> {
    /// The program the operators plan.
    pub(crate) program: LogicalProgram<'p>,
    /// Each rule's nodes, in rule order.
    pub(crate) rules: Vec<RuleNodes>,
    /// Whether each rule is served, from its plan: a step after the files or a bound input.
    pub(crate) serving: Vec<Serving>,
    /// Each node's consumers, by node.
    pub(crate) consumers: Vec<Consumers>,
    /// The nodes that start from the tree nodes of a kind, by the text the plans write for the kind.
    pub(crate) roots: hashbrown::HashMap<LiteralText, Vec<PlanNodeId>>,
    /// Each node's output columns, or the schema fault of it or of a node it reads, by node.
    pub(crate) schemas: Vec<Result<Vec<CellKind>, OperatingError>>,
}

impl<'p> LogicalBuild<'p> {
    /// The logical program, for the operators to plan.
    ///
    /// # Examples
    ///
    /// ```
    /// use ryo_execution_domain::domain::executable_program::service::logical;
    /// use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
    /// let mut inputs = ProgramInputs::default();
    /// let build = logical(&[], &mut inputs).unwrap();
    /// let _program = build.program();
    /// ```
    #[must_use]
    pub fn program(&self) -> &LogicalProgram<'p> {
        &self.program
    }
}
