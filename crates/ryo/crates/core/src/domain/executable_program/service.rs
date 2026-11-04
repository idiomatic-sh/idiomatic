//! Lower the temporary canonical analysis graph; erase internal continuation cuts.
use super::types::*;
use crate::domain::{
    file_program::types::{Node, NodeId, NodeInput, RequestGraph},
    operating::errors::OperatingError,
    operator::contract::{
        resources::ResourceProgram,
        types::{DemandOrder, Effects, InputShape, ModeContract, Publication, SchemaBoundary},
    },
};
use ryo_values::domain::{
    plan_parts::types::{Input, RepeatedRows, StepId},
    plans::types::{Operation, Plan, StepScope},
};

/// Explicit validation and fault points remain code, not implicit runtime defaults.
pub(crate) fn demand(mode: ModeContract, used: usize, join: HeldJoinOrder) -> DemandProgram {
    let mut actions = Vec::new();
    match mode.schema_boundary {
        SchemaBoundary::BeforeDemand => actions.push(DemandAction::ValidateBeforeInputs),
        SchemaBoundary::FirstApplicableRow => actions.push(DemandAction::ValidateFirstRow),
    }
    match mode.demand {
        DemandOrder::None => {}
        DemandOrder::Unary => actions.push(DemandAction::Admit(InputId(0))),
        DemandOrder::LeftThenRight => {
            actions.extend((0..used).map(|input| DemandAction::Admit(InputId(input))))
        }
        DemandOrder::LeftThenRightUnlessEmptyLeft => {
            actions.push(DemandAction::Admit(InputId(0)));
            actions.push(DemandAction::EmptyGate(InputId(0)));
            actions.push(DemandAction::Admit(InputId(1)));
        }
        DemandOrder::JoinTransport => actions.push(DemandAction::JoinInputs { held: join }),
        DemandOrder::AxisTransport => actions.push(DemandAction::AxisInputs),
    }
    if mode.publication == Publication::AfterSuccess {
        actions.push(DemandAction::PublishAfterSuccess);
    }
    DemandProgram {
        actions: actions.into_boxed_slice(),
    }
}
struct DraftRegion {
    phase: StepScope,
    nodes: Vec<usize>,
    instructions: Vec<Instruction>,
    inputs: Vec<NodeInput>,
    needs: Result<Vec<crate::domain::trust::types::RelationRead>, OperatingError>,
}
fn continuation(node: &Node) -> bool {
    node.schema_valid
        && node.contract.effects == Effects::Pure
        && matches!(node.contract.resources, ResourceProgram::None)
        && matches!(node.contract.collected_resources, ResourceProgram::None)
        && node.contract.derived.streaming.iter().all(Option::is_none)
        && node.contract.derived.collected.iter().all(Option::is_none)
        && matches!(node.contract.streaming.shape, InputShape::Row { .. })
        && node.contract.streaming.publication == Publication::Incremental
        && match &node.operation {
            Operation::Select { kept, .. } => kept.is_empty(),
            Operation::Project { distinct, .. } => *distinct == RepeatedRows::Kept,
            _ => false,
        }
}
fn root(
    plan: &Plan,
    graph: &RequestGraph,
    rule: usize,
    step: StepId,
) -> Result<usize, OperatingError> {
    let _ = plan;
    graph
        .program_nodes
        .get(rule)
        .and_then(|nodes| nodes.get(step.0 as usize))
        .ok_or(OperatingError::NotRun)?
        .as_ref()
        .map(|id| id.0)
        .map_err(Clone::clone)
}
/// The analysis graph is consumed. Its native regions, layouts, cuts, liveness
/// tables and original-plan execution copies do not survive into this program.
pub(crate) fn compile(
    plans: &[Plan],
    graph: RequestGraph,
    partitions: &dyn ProducerPartitions,
) -> Result<ExecutableProgram, OperatingError> {
    let count = graph.nodes.len();
    let mut consumers = vec![0usize; count];
    let mut origins = vec![None; count];
    for (rule, nodes) in graph.program_nodes.iter().enumerate() {
        for node in nodes.iter().filter_map(|node| node.as_ref().ok()) {
            origins[node.0].get_or_insert(crate::domain::coverage::types::RuleIndex(rule));
        }
    }
    let mut boundary = graph.derived_sources.clone();
    for node in &graph.nodes {
        for input in &node.inputs {
            if let NodeInput::Node(child) = input {
                consumers[child.0] += 1;
            }
        }
    }
    for (rule, plan) in plans.iter().enumerate() {
        if let Ok(id) = root(plan, &graph, rule, plan.emit.step) {
            boundary[id] = true;
            consumers[id] += 1;
        }
        for input in plan.frontier.iter().chain(
            plan.observations
                .iter()
                .map(|observation| &observation.source),
        ) {
            if let Input::Step(step) = input {
                if let Ok(id) = root(plan, &graph, rule, *step) {
                    boundary[id] = true;
                    consumers[id] += 1;
                }
            }
        }
    }
    // Every named publication is a true owner-issued identity, even without an Emit reader.
    if let Some(observations) = &graph.observations {
        for output in observations.outputs.iter().flatten() {
            if let crate::domain::file_program::types::GraphObservationRoot::PerFile(id) =
                output.root
            {
                boundary[id.0] = true;
            }
        }
    }
    let mut drafts: Vec<Option<DraftRegion>> = Vec::new();
    let mut mapping = vec![usize::MAX; count];
    for (logical, mut node) in graph.nodes.into_iter().enumerate() {
        // PhysicalRegion's plans/layouts have no place in executable state.
        node.regions.clear();
        let fuse = if continuation(&node) && node.inputs.len() == 1 {
            match node.inputs[0] {
                NodeInput::Node(child) if !boundary[child.0] && consumers[child.0] == 1 => {
                    let region = mapping[child.0];
                    drafts
                        .get(region)
                        .and_then(Option::as_ref)
                        .filter(|region| region.phase == node.phase)
                        .map(|_| region)
                }
                _ => None,
            }
        } else {
            None
        };
        let rule = origins[logical].ok_or(OperatingError::NotRun)?;
        let partition = match &node.operation {
            Operation::Hosted { relation } => partitions.hosted(rule, *relation)?,
            Operation::Rows { parameter, .. } => partitions.parameter(rule, *parameter)?,
            _ => PartitionRequirement::InputLineage,
        };
        let join = if node.native_join_right_first {
            HeldJoinOrder::RightBeforeLeft
        } else {
            HeldJoinOrder::LeftWithEmptyGate
        };
        let instruction = Instruction {
            logical: LogicalInstruction(logical),
            producer: rule,
            operation: node.operation,
            streaming_demand: demand(node.contract.streaming, node.contract.inputs.used.0, join),
            collected_demand: demand(node.contract.collected, node.contract.inputs.used.0, join),
            partition,
            input_delivery: node.input_supply.into_boxed_slice(),
            inputs: if fuse.is_some() {
                vec![OperandSource::PreviousRow]
            } else {
                (0..node.inputs.len())
                    .map(|at| OperandSource::Boundary(InputId(at)))
                    .collect()
            }
            .into_boxed_slice(),
            contract: node.contract,
        };
        let region = if let Some(region) = fuse {
            let region_body = drafts[region].as_mut().ok_or(OperatingError::NotRun)?;
            region_body.nodes.push(logical);
            region_body.instructions.push(instruction);
            region_body.needs = node.needs;
            region
        } else {
            let region = drafts.len();
            drafts.push(Some(DraftRegion {
                phase: node.phase,
                nodes: vec![logical],
                instructions: vec![instruction],
                inputs: node.inputs,
                needs: node.needs,
            }));
            region
        };
        mapping[logical] = region;
    }
    let mut dense = vec![usize::MAX; drafts.len()];
    let mut next = 0;
    for (at, region) in drafts.iter().enumerate() {
        if region.is_some() {
            dense[at] = next;
            next += 1;
        }
    }
    let logical_regions: Vec<RegionId> = mapping
        .iter()
        .map(|draft| RegionId(dense[*draft]))
        .collect();
    let source = |input: NodeInput| -> InputSource {
        match input {
            NodeInput::Relation(relation) => InputSource::Relation(relation),
            NodeInput::Node(id) => InputSource::Region(logical_regions[id.0]),
            NodeInput::Observation(id) => InputSource::Observation(id.0),
        }
    };
    let mut regions = Vec::with_capacity(next);
    for (at, draft) in drafts.into_iter().enumerate() {
        let Some(draft) = draft else { continue };
        let logical = *draft.nodes.last().ok_or(OperatingError::NotRun)?;
        regions.push(Region {
            id: RegionId(dense[at]),
            producer: draft
                .instructions
                .last()
                .ok_or(OperatingError::NotRun)?
                .producer,
            needs: draft.needs.map(Vec::into_boxed_slice),
            phase: draft.phase,
            instructions: draft.instructions.into_boxed_slice(),
            inputs: draft
                .inputs
                .into_iter()
                .map(source)
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            consumers: consumers[logical],
            publishes: boundary[logical] || consumers[logical] > 1,
        });
    }
    let mut rules = Vec::with_capacity(plans.len());
    for (rule, (plan, nodes)) in plans.iter().zip(graph.program_nodes).enumerate() {
        let original_steps: Vec<_> = nodes
            .into_iter()
            .enumerate()
            .map(|(step, node)| {
                node.map(|id| Origin {
                    rule: crate::domain::coverage::types::RuleIndex(rule),
                    step: StepId(step as u32),
                    logical: LogicalInstruction(id.0),
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
                    let slot = graph
                        .observations
                        .as_ref()
                        .ok_or_else(missing)?
                        .slots
                        .iter()
                        .position(|slot| slot.rule == rule && slot.input == *id)
                        .ok_or_else(missing)?;
                    Ok(InputSource::Observation(slot))
                }
            }
        };
        let emit = from(&Input::Step(plan.emit.step)).and_then(|source| match source {
            InputSource::Region(region) => Ok(region),
            _ => Err(OperatingError::WrongOperation),
        });
        rules.push(RuleProgram {
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
    let fused_instruction_count = count - regions.len();
    Ok(ExecutableProgram {
        regions: regions.into_boxed_slice(),
        rules: rules.into_boxed_slice(),
        logical_regions: logical_regions.into_boxed_slice(),
        original_instruction_count: count,
        fused_instruction_count,
    })
}

#[cfg(test)]
#[path = "observation_slot_test.rs"]
mod observation_slot_test;
