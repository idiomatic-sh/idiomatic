//! Sole region invocation lifecycle. Internal continuations have no publication,
//! room, dependency ledger or separate scheduler state.
use super::admission::{AdmittedInput, Inputs, Partition};
use crate::domain::{
    executable_program::types::{
        DemandAction, ExecutableProgram, InputId, Instruction, Region, RegionId,
    },
    operating::{
        errors::OperatingError,
        types::{Emit, OperatorContext},
    },
    table_scratch::types::OperatorScratch,
};
use ryo_relations::domain::relations::types::Value;
use std::{cell::RefCell, sync::Arc};

pub(super) enum Delivery<'a> {
    Retain,
    Forward(&'a mut dyn Emit),
}
pub(super) enum InvocationFailure {
    Semantic(OperatingError),
    Delivery(OperatingError),
}
/// No held inputs/publication or dependency accounting at internal instructions.
#[derive(Default)]
pub(super) struct KernelScratch {
    pub scratch: OperatorScratch,
    pub memo: Vec<crate::domain::operating::types::Memo>,
}
pub(super) enum KernelInput<'a> {
    Admitted(&'a [AdmittedInput]),
    Continuation(&'a [Value]),
}
/// Application-owned preparation/acquisition supplies the borrowed semantic call.
/// All implementations are required; no missing-capability or fallback path.
pub(super) trait KernelInvocation {
    fn evaluate(
        &self,
        instruction: &Instruction,
        permit: &super::admission::KernelPermit<'_>,
        input: KernelInput<'_>,
        context: &mut OperatorContext<'_>,
        scratch: &mut KernelScratch,
        output: &mut dyn Emit,
    ) -> Result<(), InvocationFailure>;
}
enum State {
    Dormant,
    Running,
    Published(AdmittedInput),
    Forwarded,
    Failed(OperatingError),
}
struct Slot {
    state: State,
    scratch: Vec<KernelScratch>,
    remaining: usize,
}
pub(crate) struct ProgramExecution {
    pub(super) program: Arc<ExecutableProgram>,
    pub(super) owner: std::rc::Rc<super::super::relations::RelationOwner>,
    pub(super) generation: Arc<()>,
    partition: Partition,
    slots: RefCell<Vec<Slot>>,
}
/// Constructors are private to the outer execution owner; no primitive can open
/// another scope or choose a scheduler.
impl ProgramExecution {
    pub(in crate::application::file_dag::program) fn open(
        program: Arc<ExecutableProgram>,
        partition: Partition,
        owner: std::rc::Rc<super::super::relations::RelationOwner>,
    ) -> Self {
        let slots = program
            .regions
            .iter()
            .map(|region| Slot {
                state: State::Dormant,
                scratch: (0..region.instructions.len())
                    .map(|_| KernelScratch::default())
                    .collect(),
                remaining: region.consumers,
            })
            .collect();
        let generation = Arc::clone(&owner.generation.borrow());
        Self {
            program,
            owner,
            generation,
            partition,
            slots: RefCell::new(slots),
        }
    }
    pub(in crate::application::file_dag::program) fn invoke(
        &self,
        id: RegionId,
        inputs: &mut dyn Inputs,
        kernels: &dyn KernelInvocation,
        context: &mut OperatorContext<'_>,
        delivery: Delivery<'_>,
    ) -> Result<InvocationOutcome, OperatingError> {
        let region = self
            .program
            .regions
            .get(id.0)
            .ok_or(OperatingError::NotRun)?;
        // A scope borrow never crosses admission, kernels or downstream callbacks.
        let mut scratch = {
            let mut slots = self.slots.borrow_mut();
            let slot = slots.get_mut(id.0).ok_or(OperatingError::NotRun)?;
            match &slot.state {
                State::Running => return Err(OperatingError::NotRun),
                State::Failed(error) => return Err(error.clone()),
                State::Published(input) => {
                    let input = input.clone();
                    drop(slots);
                    return match delivery {
                        Delivery::Retain => Ok(InvocationOutcome::Published(input)),
                        Delivery::Forward(output) => {
                            inputs.replay(&input, context, output)?;
                            Ok(InvocationOutcome::Forwarded)
                        }
                    };
                }
                State::Forwarded => return Err(OperatingError::NotRun),
                State::Dormant => {}
            }
            slot.state = State::Running;
            std::mem::take(&mut slot.scratch)
        };
        let mut cleanup = Invocation {
            execution: self,
            id,
            scratch: Some(scratch),
            terminal: Terminal::Retry,
        };
        let first = region.instructions.first().ok_or(OperatingError::NotRun)?;
        let mut admitted = Vec::new();
        let program = match delivery {
            Delivery::Retain => &first.collected_demand,
            Delivery::Forward(_) => &first.streaming_demand,
        };
        let result: Result<InvocationOutcome, InvocationFailure> = (|| {
            if matches!(delivery, Delivery::Retain) {
                for instruction in region.instructions.iter().rev() {
                    if instruction.contract.collected.schema_boundary
                        == crate::domain::operator::contract::types::SchemaBoundary::BeforeDemand
                    {
                        instruction
                            .contract
                            .collected_kinds
                            .as_ref()
                            .map_err(|error| InvocationFailure::Semantic(error.clone()))?;
                    }
                }
            }
            for action in program.actions() {
                match *action {
                    DemandAction::ValidateBeforeInputs => {
                        first
                            .contract
                            .schema
                            .as_ref()
                            .map_err(|error| InvocationFailure::Semantic(error.clone()))?;
                    }
                    DemandAction::ValidateFirstRow | DemandAction::PublishAfterSuccess => {}
                    DemandAction::Admit(InputId(at)) => {
                        let source = *region.inputs.get(at).ok_or_else(|| {
                            InvocationFailure::Semantic(OperatingError::MissingInput {
                                position: at + 1,
                            })
                        })?;
                        admitted.push(
                            inputs
                                .admit(source, &self.partition)
                                .map_err(InvocationFailure::Semantic)?,
                        );
                    }
                    DemandAction::EmptyGate(InputId(at)) => {
                        if inputs
                            .is_empty(admitted.get(at).ok_or_else(|| {
                                InvocationFailure::Semantic(OperatingError::MissingInput {
                                    position: at + 1,
                                })
                            })?)
                            .map_err(InvocationFailure::Semantic)?
                        {
                            let published = inputs
                                .publish_empty(
                                    super::admission::ProducerRoot::Region(id),
                                    &self.partition,
                                    &admitted,
                                    region
                                        .instructions
                                        .last()
                                        .ok_or_else(|| {
                                            InvocationFailure::Semantic(OperatingError::NotRun)
                                        })?
                                        .contract
                                        .collected_kinds
                                        .clone(),
                                )
                                .map_err(InvocationFailure::Semantic)?;
                            return Ok(InvocationOutcome::Published(published));
                        }
                    }
                    DemandAction::JoinInputs { .. } => {
                        admitted = admit_join(region, first, inputs, &self.partition, context)
                            .map_err(InvocationFailure::Semantic)?
                    }
                    DemandAction::AxisInputs => {
                        admitted = admit_axis(region, first, inputs, &self.partition, context)
                            .map_err(InvocationFailure::Semantic)?
                    }
                }
            }
            let frames = cleanup
                .scratch
                .as_mut()
                .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
            let last = region
                .instructions
                .last()
                .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
            match delivery {
                Delivery::Forward(output) if !region.publishes => {
                    let mut output = GuardedOutput {
                        output,
                        error: None,
                    };
                    let result = continuation(
                        &permit,
                        region,
                        0,
                        KernelInput::Admitted(&admitted),
                        kernels,
                        context,
                        frames,
                        &mut output,
                    );
                    if let Some(error) = output.error {
                        return Err(InvocationFailure::Delivery(error));
                    }
                    result?;
                    Ok(InvocationOutcome::Forwarded)
                }
                delivery => {
                    let mut rows = ryo_values::domain::rows::types::Table::default();
                    crate::domain::table_model::service::reset(
                        &mut rows,
                        last.contract
                            .collected_kinds
                            .as_ref()
                            .map_err(|error| InvocationFailure::Semantic(error.clone()))?,
                    );
                    match delivery {
                        Delivery::Retain => {
                            let mut output = crate::domain::replay::types::Collect(&mut rows);
                            continuation(
                                &permit,
                                region,
                                0,
                                KernelInput::Admitted(&admitted),
                                kernels,
                                context,
                                frames,
                                &mut output,
                            )?;
                        }
                        Delivery::Forward(output) => {
                            let mut output = Tee {
                                rows: &mut rows,
                                output,
                                error: None,
                            };
                            let result = continuation(
                                &permit,
                                region,
                                0,
                                KernelInput::Admitted(&admitted),
                                kernels,
                                context,
                                frames,
                                &mut output,
                            );
                            if let Some(error) = output.error {
                                return Err(InvocationFailure::Delivery(error));
                            }
                            result?;
                        }
                    }
                    let published = inputs
                        .publish(
                            super::admission::ProducerRoot::Region(id),
                            &self.partition,
                            &admitted,
                            rows,
                        )
                        .map_err(InvocationFailure::Semantic)?;
                    Ok(InvocationOutcome::Published(published))
                }
            }
        })();
        match result {
            Ok(InvocationOutcome::Published(input)) => {
                cleanup.terminal = Terminal::Published(input.clone());
                Ok(InvocationOutcome::Published(input))
            }
            Ok(InvocationOutcome::Forwarded) => {
                cleanup.terminal = Terminal::Forwarded;
                Ok(InvocationOutcome::Forwarded)
            }
            Err(InvocationFailure::Semantic(error)) => {
                cleanup.terminal = Terminal::Failed(error.clone());
                Err(error)
            }
            Err(InvocationFailure::Delivery(error)) => Err(error),
        }
    }
}
fn continuation(
    invocation: &super::admission::KernelPermit<'_>,
    region: &Region,
    at: usize,
    input: KernelInput<'_>,
    kernels: &dyn KernelInvocation,
    context: &mut OperatorContext<'_>,
    scratch: &mut [KernelScratch],
    output: &mut dyn Emit,
) -> Result<(), InvocationFailure> {
    let instruction = region
        .instructions
        .get(at)
        .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
    let permit = invocation
        .instruction(at)
        .map_err(InvocationFailure::Semantic)?;
    let (frame, rest) = scratch
        .split_first_mut()
        .ok_or_else(|| InvocationFailure::Semantic(OperatingError::NotRun))?;
    if at + 1 == region.instructions.len() {
        return kernels.evaluate(instruction, &permit, input, context, frame, output);
    }
    let mut next = Continue {
        invocation,
        region,
        at: at + 1,
        kernels,
        scratch: rest,
        output,
    };
    kernels.evaluate(instruction, &permit, input, context, frame, &mut next)
}
struct Continue<'a> {
    invocation: &'a super::admission::KernelPermit<'a>,
    region: &'a Region,
    at: usize,
    kernels: &'a dyn KernelInvocation,
    scratch: &'a mut [KernelScratch],
    output: &'a mut dyn Emit,
}
impl Emit for Continue<'_> {
    fn row(
        &mut self,
        context: &mut OperatorContext<'_>,
        row: &[Value],
    ) -> Result<(), OperatingError> {
        continuation(
            self.invocation,
            self.region,
            self.at,
            KernelInput::Continuation(row),
            self.kernels,
            context,
            self.scratch,
            self.output,
        )
        .map_err(|failure| match failure {
            InvocationFailure::Semantic(error) | InvocationFailure::Delivery(error) => error,
        })
    }
}
struct GuardedOutput<'a> {
    output: &'a mut dyn Emit,
    error: Option<OperatingError>,
}
impl Emit for GuardedOutput<'_> {
    fn row(
        &mut self,
        context: &mut OperatorContext<'_>,
        row: &[Value],
    ) -> Result<(), OperatingError> {
        let result = self.output.row(context, row);
        if let Err(error) = &result {
            if self.error.is_none() {
                self.error = Some(error.clone());
            }
        }
        result
    }
}
pub(super) enum InvocationOutcome {
    Published(AdmittedInput),
    Forwarded,
}
enum Terminal {
    Retry,
    Published(AdmittedInput),
    Forwarded,
    Failed(OperatingError),
}
struct Invocation<'a> {
    execution: &'a ProgramExecution,
    id: RegionId,
    scratch: Option<Vec<KernelScratch>>,
    terminal: Terminal,
}
impl Drop for Invocation<'_> {
    fn drop(&mut self) {
        let mut slots = self.execution.slots.borrow_mut();
        if let Some(slot) = slots.get_mut(self.id.0) {
            slot.scratch = self.scratch.take().unwrap_or_default();
            slot.state = match &self.terminal {
                Terminal::Retry => State::Dormant,
                Terminal::Published(input) => State::Published(input.clone()),
                Terminal::Forwarded => State::Forwarded,
                Terminal::Failed(error) => State::Failed(error.clone()),
            };
        }
    }
}

fn admit_join(
    region: &Region,
    instruction: &Instruction,
    inputs: &mut dyn Inputs,
    partition: &Partition,
    context: &mut OperatorContext<'_>,
) -> Result<Vec<AdmittedInput>, OperatingError> {
    use crate::domain::{executable_program::types::HeldJoinOrder, operating::types::Supply};
    let supply = |at| {
        instruction
            .input_delivery
            .get(at)
            .copied()
            .ok_or(OperatingError::MissingInput { position: at + 1 })
    };
    let source = |at| {
        region
            .inputs
            .get(at)
            .copied()
            .ok_or(OperatingError::MissingInput { position: at + 1 })
    };
    let held = instruction
        .streaming_demand
        .actions()
        .iter()
        .find_map(|action| match action {
            DemandAction::JoinInputs { held } => Some(*held),
            _ => None,
        })
        .ok_or(OperatingError::WrongOperation)?;
    if supply(0)? == Supply::AtHand || supply(1)? == Supply::Streamed {
        match held {
            HeldJoinOrder::RightBeforeLeft => {
                let right = inputs.admit(source(1)?, partition, context)?;
                let left = inputs.admit(source(0)?, partition, context)?;
                Ok(vec![left, right])
            }
            HeldJoinOrder::LeftWithEmptyGate => {
                let left = inputs.admit(source(0)?, partition, context)?;
                if inputs.is_empty(&left)? {
                    return Ok(vec![left]);
                }
                let right = inputs.admit(source(1)?, partition, context)?;
                Ok(vec![left, right])
            }
        }
    } else {
        let right = inputs.admit(source(1)?, partition, context)?;
        let left = inputs.admit(source(0)?, partition, context)?;
        Ok(vec![left, right])
    }
}
fn admit_axis(
    region: &Region,
    instruction: &Instruction,
    inputs: &mut dyn Inputs,
    partition: &Partition,
    context: &mut OperatorContext<'_>,
) -> Result<Vec<AdmittedInput>, OperatingError> {
    // The explicit ordinal contract is preserved even when a producer is supplied as a stream.
    let mut admitted = Vec::with_capacity(instruction.input_delivery.len());
    for at in 0..instruction.input_delivery.len() {
        admitted.push(
            inputs.admit(
                *region
                    .inputs
                    .get(at)
                    .ok_or(OperatingError::MissingInput { position: at + 1 })?,
                partition,
                context,
            )?,
        );
    }
    Ok(admitted)
}

struct Tee<'a> {
    rows: &'a mut ryo_values::domain::rows::types::Table,
    output: &'a mut dyn Emit,
    error: Option<OperatingError>,
}
impl Emit for Tee<'_> {
    fn row(
        &mut self,
        context: &mut OperatorContext<'_>,
        row: &[Value],
    ) -> Result<(), OperatingError> {
        self.rows.cells.extend_from_slice(row);
        self.rows.count.0 += 1;
        let result = self.output.row(context, row);
        if let Err(error) = &result {
            if self.error.is_none() {
                self.error = Some(error.clone());
            }
        }
        result
    }
}
