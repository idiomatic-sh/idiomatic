//! One part of the executable-program service: where each original step of each rule is in the program, and what each rule
//! reads as its roots.
use super::types::region::value::Region;
use super::types::{
    InputSource, InstructionPlace, LogicalInstruction, ObservationPlace, RegionId, RuleProgram,
    RulePrograms, StepOrigin, StepSite, StepUse,
};
use crate::domain::{
    coverage::types::RuleIndex,
    file_program::types::{GraphObservations, ProgramNodeId},
    operating::errors::OperatingError,
};
use ryo_values::domain::{
    plan_parts::types::{Input, StepId},
    plans::service::{inputs_of, named},
    plans::types::Plan,
};

/// Whether each step of a plan is read by one of its roots (its emit step, a frontier step or a named output), through the
/// steps that read it.
fn unread_steps(plan: &Plan) -> Vec<StepUse> {
    let mut uses = vec![StepUse::Unread; plan.steps.len()];
    let mut pending: Vec<StepId> = std::iter::once(Input::Step(plan.emit.step))
        .chain(plan.frontier.iter().copied())
        .chain(plan.observations.iter().map(|output| output.source))
        .filter_map(|input| match input {
            Input::Step(step) => Some(step),
            Input::Relation(_) | Input::BoundObservation(_) => None,
        })
        .collect();
    while let Some(step) = pending.pop() {
        let at = step.0 as usize;
        let Some(use_) = uses.get_mut(at) else {
            continue;
        };
        if *use_ == StepUse::Read {
            continue;
        }
        *use_ = StepUse::Read;
        if let Some(read) = plan.steps.get(at) {
            for input in named(&inputs_of(&read.operation)) {
                if let Input::Step(earlier) = input {
                    pending.push(*earlier);
                }
            }
        }
    }
    uses
}

/// Where `logical` is: its region and its place among the region's instructions, when the program has it.
fn site(
    logical: LogicalInstruction,
    logical_regions: &[RegionId],
    regions: &[Region],
) -> Option<StepSite> {
    let region = *logical_regions.get(logical.0)?;
    let at = regions
        .get(region.0)?
        .instructions()
        .iter()
        .position(|instruction| instruction.logical() == logical)?;
    Some(StepSite {
        region,
        instruction: InstructionPlace(at),
    })
}

/// Places each original step of `rules` in the finished program: the region of its logical instruction and the instruction's
/// place there. Run once the regions are final, after the unions are appended and the aliases resolved, so a place is what the
/// lookup over the finished program finds.
pub(super) fn place_steps(
    rules: &mut [RuleProgram],
    logical_regions: &[RegionId],
    regions: &[Region],
) {
    for origin in rules
        .iter_mut()
        .flat_map(|rule| rule.original_steps.iter_mut())
        .flatten()
    {
        origin.place = site(origin.logical, logical_regions, regions);
    }
}

/// The program of every rule, and the region each plan's named output reads, in the plan's own output order. A step's
/// origin is refused when the step has no node (it reads a step that has not run) or when no root reads it and its schema
/// is at fault (`faults`, by original node).
pub(super) fn rule_programs(
    plans: &[Plan],
    program_nodes: Vec<Vec<Result<ProgramNodeId, OperatingError>>>,
    observations: Option<&GraphObservations>,
    representative: &[LogicalInstruction],
    logical_regions: &[RegionId],
    faults: &[Option<OperatingError>],
) -> RulePrograms {
    let mut rules = Vec::with_capacity(plans.len());
    // The region each plan's named output reads, in the plan's own output order.
    let mut observed_roots: Vec<Vec<Option<RegionId>>> = Vec::with_capacity(plans.len());
    for (rule, (plan, nodes)) in plans.iter().zip(program_nodes).enumerate() {
        let unread = unread_steps(plan);
        let original_steps: Vec<_> = nodes
            .into_iter()
            .enumerate()
            .map(|(step, node)| {
                let id = node?;
                // A step no root reads has no root to deliver its schema fault to: its origin is the fault, so the
                // rule is refused on its own roots instead of the fault being dropped.
                if let (Some(StepUse::Unread), Some(Some(fault))) =
                    (unread.get(step), faults.get(id.0))
                {
                    return Err(fault.clone());
                }
                let logical = representative[id.0];
                Ok(StepOrigin {
                    rule: RuleIndex(rule),
                    step: StepId(u32::try_from(step).unwrap_or(u32::MAX)),
                    logical,
                    place: None,
                })
            })
            .collect();
        let from = |input: &Input| -> Result<InputSource, OperatingError> {
            match input {
                Input::Relation(relation) => Ok(InputSource::Relation(*relation)),
                Input::Step(step) => original_steps
                    .get(step.0 as usize)
                    .ok_or(OperatingError::NotRun)?
                    .as_ref()
                    .map(|origin| InputSource::Region(logical_regions[origin.logical.0]))
                    .map_err(Clone::clone),
                Input::BoundObservation(id) => {
                    let missing = || OperatingError::ObservationIncomplete {
                        reason: format!(
                            "rule {rule} observation input {} has no request-qualified slot",
                            id.0
                        ),
                    };
                    plan.observation_inputs
                        .get(id.0 as usize)
                        .ok_or_else(missing)?;
                    let slot = observations
                        .ok_or_else(missing)?
                        .slots
                        .iter()
                        .position(|slot| slot.rule.0 == rule && slot.input == *id)
                        .ok_or_else(missing)?;
                    Ok(InputSource::Observation(ObservationPlace(slot)))
                }
            }
        };
        let emit = from(&Input::Step(plan.emit.step)).and_then(|source| match source {
            InputSource::Region(region) => Ok(region),
            _ => Err(OperatingError::WrongOperation),
        });
        observed_roots.push(
            plan.observations
                .iter()
                .map(|output| match from(&output.source) {
                    Ok(InputSource::Region(id)) => Some(id),
                    _ => None,
                })
                .collect(),
        );
        let root_readers = emit
            .iter()
            .copied()
            .chain(
                plan.frontier
                    .iter()
                    .chain(plan.observations.iter().map(|output| &output.source))
                    .filter_map(|input| match input {
                        Input::Step(_) => match from(input) {
                            Ok(InputSource::Region(id)) => Some(id),
                            _ => None,
                        },
                        _ => None,
                    }),
            )
            .collect::<Vec<_>>()
            .into_boxed_slice();
        rules.push(RuleProgram {
            root_readers,
            emit,
            frontier: plan
                .frontier
                .iter()
                .map(from)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            original_steps: original_steps.into_boxed_slice(),
        });
    }
    RulePrograms {
        rules,
        observed_roots,
    }
}
