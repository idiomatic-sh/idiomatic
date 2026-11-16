//! How a rule's node answers in either pass: run as its answer needs (a refusal answered after its inputs, without its kernel),
//! its rows read (a refusal deferred to its rows answering its drafted layout), and what its kernels read of each input when
//! it ran. The one check and the one count of the file pass and the project pass.

use crate::application::file_dag::file_pass::types::NodeMove;
use crate::application::file_dag::frontiers::service::reason;
use crate::application::file_dag::frontiers::types::Withholdings;
use ryo_execution_domain::domain::executable_program::service::refusals::service::{
    deferred_layout, layout, node_inputs,
};
use ryo_execution_domain::domain::executable_program::types::executable_program::value::ExecutableProgram;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operator::contract::service::indexed_input;
use ryo_execution_domain::domain::operator::types::RowsFed;
use ryo_operators::domain::physical_plan::types::file_buffers::value::FileBuffers;
use ryo_operators::domain::physical_plan::types::held_rows::value::HeldRows;
use ryo_operators::domain::physical_plan::types::{NodeFed, NodeRows, PlanInput, PlanNodeId};
use ryo_values::domain::plans::service::{inputs_of, named};
use ryo_values::domain::plans::types::PlanStep;
use ryo_values::domain::rows::service::reset;
use ryo_values::domain::rows::types::Table;

/// Runs `node` as its answer needs, each move through `step`, the pass's own: when it answers rows, each deferred refusal it reads
/// the same way, then answering no column when it holds no row (`NodeMove::Empty`), and then the node itself (`NodeMove::Run`); for a node whose refusal is answered after its inputs, each input in the order it reads them, a node the
/// same way and any other read (`NodeMove::Read`: a relation's facts admitted, a frontier entry or a bound observation held),
/// so a refusal or a withholding of theirs comes first, and then its own refusal, without running its kernel. The one check of
/// both passes, in each file and after the files.
///
/// # Errors
///
/// The first refusal or withholding of the node's inputs, then the node's own (`layout`); or what `step` answered.
pub(crate) fn ran_answering(
    program: &ExecutableProgram,
    node: PlanNodeId,
    step: &mut dyn FnMut(NodeMove) -> Result<(), OperatingError>,
) -> Result<(), OperatingError> {
    match layout(program, node) {
        Ok(_) => {
            // A node reading a deferred refusal runs its node inputs first, in the order it reads them, up to the last
            // deferred one, as the interpreter ran a step's inputs before the step: an input's row error comes before any
            // later input runs, and a deferred refusal holding no row answers the node no column.
            let inputs = node_inputs(program, node);
            let deferred = |input: &PlanInput| match input {
                PlanInput::Node(read) => deferred_layout(program, *read).is_some(),
                PlanInput::Relation(_) | PlanInput::Frontier(_) | PlanInput::Observation(_) => {
                    false
                }
            };
            let before = inputs.iter().rposition(deferred).map_or(0, |last| last + 1);
            for input in inputs.get(..before).unwrap_or_default() {
                if let PlanInput::Node(read) = input {
                    ran_answering(program, *read, step)?;
                    if deferred(input) {
                        step(NodeMove::Empty(*read))?;
                    }
                }
            }
            step(NodeMove::Run(node))
        }
        Err(refusal) => {
            for input in node_inputs(program, node) {
                match input {
                    PlanInput::Node(read) => ran_answering(program, *read, step)?,
                    PlanInput::Relation(_) | PlanInput::Frontier(_) | PlanInput::Observation(_) => {
                        step(NodeMove::Read(*input))?;
                    }
                }
            }
            Err(refusal)
        }
    }
}

/// The rows a node answered once [`ran_answering`] ran it: a node whose refusal is deferred to its rows and that no row reached
/// answers with its drafted layout, whatever its kernel read from an input that defers its own.
///
/// # Errors
///
/// `NotRun` for a node that holds no rows.
pub(crate) fn ran_rows(
    program: &ExecutableProgram,
    buffers: &FileBuffers,
    node: PlanNodeId,
) -> Result<Table, OperatingError> {
    match buffers.rows(node) {
        NodeRows::Materialized(table) => match deferred_layout(program, node) {
            Some(kinds) if table.count.0 == 0 => {
                let mut answered = Table::default();
                reset(&mut answered, kinds);
                Ok(answered)
            }
            Some(_) | None => Ok(table.clone()),
        },
        NodeRows::Absent => Err(OperatingError::NotRun),
    }
}

/// An input held for a reader: nothing to answer when held, why it was withheld when withheld.
///
/// # Errors
///
/// The reason recorded for the withholding.
pub(crate) fn held(withholdings: &Withholdings, rows: HeldRows<'_>) -> Result<(), OperatingError> {
    match rows {
        HeldRows::Held(_) => Ok(()),
        HeldRows::Withheld(token) => Err(reason(withholdings, token)),
    }
}

/// What the kernels of `node`, the node of plan step `step`, read of each of the step's inputs when it ran: none when it never
/// ran (no root reached it, or it answered a refusal without running its kernel), none for a slot the step names no input at
/// (a hosted step reads a request source, no input), and none for the input it builds into an index (`indexed_input`). The
/// one count of both passes.
pub(crate) fn step_fed(
    buffers: &FileBuffers,
    step: &PlanStep,
    node: &Result<PlanNodeId, OperatingError>,
) -> RowsFed {
    let Ok(node) = node else {
        return RowsFed::default();
    };
    let inputs = named(&inputs_of(&step.operation)).len();
    // An input the step builds into an index is read as a resource, never fed: its rows count nothing.
    let indexed = indexed_input(&step.operation);
    let NodeFed(rows) = buffers.fed(*node);
    let mut fed = RowsFed::default();
    for (slot, (count, each)) in fed
        .0
        .iter_mut()
        .zip(rows.iter().enumerate().map(|(at, count)| (count, at)))
    {
        if each < inputs && indexed.is_none_or(|input| input.0 != each) {
            *slot = u64::try_from(count.0).unwrap_or(u64::MAX);
        }
    }
    fed
}
