//! How a planned program's nodes answer when a schema refuses: the columns a node's rows answer with as a root, the layout of
//! a refusal deferred to its rows, and the inputs that run, are admitted or are read before a refusal answered after its
//! inputs.

use crate::domain::executable_program::types::executable_program::value::ExecutableProgram;
use crate::domain::executable_program::types::{Draft, SchemaTiming};
use crate::domain::operating::errors::OperatingError;
use ryo_operators::domain::physical_plan::types::{PlanInput, PlanNodeId};
use ryo_relations::domain::relations::types::CellKind;

/// The columns `node`'s rows answer with as a root: its schema, or, for a node whose refusal is deferred to its rows, the
/// layout its empty rows keep.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::service::{assemble, logical};
/// use ryo_execution_domain::domain::executable_program::service::refusals::service::layout;
/// use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
/// use ryo_operators::domain::physical_plan::types::{PlanInput, PlanNodeId};
/// use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
/// let mut inputs = ProgramInputs::default();
/// let program = assemble(logical(&[], &mut inputs).unwrap(), PhysicalPlan::default());
/// assert!(layout(&program, PlanNodeId(0)).is_err());
/// ```
///
/// # Errors
///
/// The refusal the node answers with, after its inputs for one that reads a deferred refusal; `NotRun` for a node the
/// program does not have.
pub fn layout(
    program: &ExecutableProgram,
    node: PlanNodeId,
) -> Result<&[CellKind], OperatingError> {
    match program
        .layouts
        .get(usize::try_from(node.0).unwrap_or(usize::MAX))
    {
        Some(Ok(kinds)) => Ok(kinds),
        Some(Err(error)) => Err(error.clone()),
        None => Err(OperatingError::NotRun),
    }
}

/// The inputs `node` reads, in the order it reads them: each runs, is admitted or is read, before its refusal, when its refusal
/// is answered after its inputs; a withheld one answers why it was withheld first.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::service::{assemble, logical};
/// use ryo_execution_domain::domain::executable_program::service::refusals::service::node_inputs;
/// use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
/// use ryo_operators::domain::physical_plan::types::{PlanInput, PlanNodeId};
/// use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
/// let mut inputs = ProgramInputs::default();
/// let program = assemble(logical(&[], &mut inputs).unwrap(), PhysicalPlan::default());
/// assert!(node_inputs(&program, PlanNodeId(0)).is_empty());
/// ```
#[must_use]
pub fn node_inputs(program: &ExecutableProgram, node: PlanNodeId) -> &[PlanInput] {
    program
        .reads
        .get(usize::try_from(node.0).unwrap_or(usize::MAX))
        .map_or(&[], Vec::as_slice)
}

/// The layout `node` answers with as a root when its refusal is deferred to its rows and no row reached it: the columns its
/// drafting gave it, whatever its kernel read from an input that defers its own refusal. None for any other node.
///
/// # Examples
///
/// ```
/// use ryo_execution_domain::domain::executable_program::service::{assemble, logical};
/// use ryo_execution_domain::domain::executable_program::service::refusals::service::deferred_layout;
/// use ryo_execution_domain::domain::executable_program::types::program_inputs::value::ProgramInputs;
/// use ryo_operators::domain::physical_plan::types::{PlanInput, PlanNodeId};
/// use ryo_operators::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
/// let mut inputs = ProgramInputs::default();
/// let program = assemble(logical(&[], &mut inputs).unwrap(), PhysicalPlan::default());
/// assert!(deferred_layout(&program, PlanNodeId(0)).is_none());
/// ```
#[must_use]
pub fn deferred_layout(program: &ExecutableProgram, node: PlanNodeId) -> Option<&[CellKind]> {
    program
        .deferred
        .get(usize::try_from(node.0).unwrap_or(usize::MAX))
        .and_then(Option::as_deref)
}

/// The inputs each node reads, by node, in the order its input `lists` name them.
pub(in crate::domain::executable_program) fn inputs_by_node(
    lists: &[Vec<PlanInput>],
) -> Vec<Vec<PlanInput>> {
    lists.to_vec()
}

/// The layout of each drafted node whose refusal is deferred to its rows, by node; none for any other.
pub(in crate::domain::executable_program) fn deferred_layouts(
    drafts: &[Draft<'_>],
) -> Vec<Option<Vec<CellKind>>> {
    drafts
        .iter()
        .map(|draft| match (draft.timing, &draft.layout) {
            (SchemaTiming::DeferredToRows, Ok(kinds)) => Some(kinds.clone()),
            (SchemaTiming::DeferredToRows, Err(_))
            | (SchemaTiming::Stated | SchemaTiming::AfterInputs, _) => None,
        })
        .collect()
}
