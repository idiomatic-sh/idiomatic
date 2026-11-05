//! Lower the temporary canonical analysis graph; erase internal continuation cuts.
use super::types::*;
use crate::domain::{
    file_program::types::{FileProgramNode, NodeId, NodeInput, RequestGraph},
    operating::errors::OperatingError,
    operator::contract::{
        resources::ResourceProgram,
        types::{DemandOrder, Effects, InputShape, SemanticContract},
    },
};
use ryo_values::domain::{
    plan_parts::types::{Input, RepeatedRows, StepId},
    plans::types::{Operation, Plan, StepScope},
};

/// Explicit validation and fault points remain code, not implicit runtime defaults.
pub(crate) fn demand(contract: SemanticContract, used: usize) -> DemandProgram {
    let actions = match contract.demand {
        DemandOrder::None => Vec::new(),
        DemandOrder::Unary => vec![DemandAction::Admit(InputId(0))],
        DemandOrder::LeftThenRight => (0..used)
            .map(|input| DemandAction::Admit(InputId(input)))
            .collect(),
    };
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
fn continuation(node: &FileProgramNode) -> bool {
    node.schema_valid
        && node.contract.effects == Effects::Pure
        && matches!(node.contract.resources, ResourceProgram::None)
        && node.contract.derived.slots.iter().all(Option::is_none)
        && matches!(node.contract.semantics.shape, InputShape::Row { .. })
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
    prepared: &crate::domain::operator::contract::types::RequestOperators,
) -> Result<ExecutableProgram, OperatingError> {
    let mut column_candidates = super::column::ColumnCandidates::capture(&graph);
    let native_shapes = super::native::capture(&graph);
    let count = graph.nodes.len();
    let mut consumers = vec![0usize; count];
    let mut origins = vec![None; count];
    for (rule, nodes) in graph.program_nodes.iter().enumerate() {
        for node in nodes.iter().filter_map(|node| node.as_ref().ok()) {
            origins[node.0].get_or_insert(crate::domain::coverage::types::RuleIndex(rule));
        }
    }
    let mut boundary = graph.derived_sources.clone();
    let ordered_alternatives = graph.ordered_existence.clone();
    for alternative in ordered_alternatives.iter().flatten() {
        if let crate::domain::ordered_existence::types::ResolvedAlternative::Eligible(shape) =
            alternative
        {
            boundary[shape.join.0] = true;
            boundary[shape.select.0] = true;
        }
    }

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
    column_candidates.retain_internal(|node| {
        if consumers[node.0] == 1 && !boundary[node.0] {
            super::column::ReplacementUse::Exclusive
        } else {
            super::column::ReplacementUse::Observed
        }
    });
    for node in column_candidates.fences() {
        boundary[node.0] = true;
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
        let mut instruction = Instruction {
            logical: LogicalInstruction(logical),
            producer: rule,
            operation: node.operation,
            demand: demand(node.contract.semantics, node.contract.inputs.used.0),
            partition,
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
        if matches!(instruction.operation, Operation::Hosted { .. }) {
            for program in [&mut instruction.demand] {
                let mut actions = program.actions.to_vec();
                let at = 0;
                actions.insert(at, DemandAction::Admit(InputId(0)));
                program.actions = actions.into_boxed_slice();
            }
        }
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
        let mut inputs: Vec<_> = draft.inputs.into_iter().map(source).collect();
        if let Some(Instruction {
            producer,
            operation: Operation::Hosted { relation },
            ..
        }) = draft.instructions.first()
        {
            inputs.push(InputSource::Hosted {
                rule: *producer,
                relation: *relation,
            });
        }
        regions.push(Region {
            source_partitions: Box::new([]),
            id: RegionId(dense[at]),
            producer: draft
                .instructions
                .last()
                .ok_or(OperatingError::NotRun)?
                .producer,
            needs: draft.needs.map(Vec::into_boxed_slice),
            phase: draft.phase,
            instructions: draft.instructions.into_boxed_slice(),
            inputs: inputs.into_boxed_slice(),
            consumers: consumers[logical],
            dependencies: Box::new([]),
            ordered_existence: None,
            column: None,
            selection: None,
            native_positions: None,
        });
    }
    for column in column_candidates.lower(&logical_regions) {
        let target = column.target();
        if regions[target.0].column.is_some() {
            continue;
        }
        let left = column.left();
        regions[left.0].consumers += 1;
        regions[target.0].column = Some(column);
    }
    for alternative in ordered_alternatives.into_iter().flatten() {
        let crate::domain::ordered_existence::types::ResolvedAlternative::Eligible(resolved) =
            alternative
        else {
            continue;
        };
        let select = logical_regions[resolved.select.0];
        let source = logical_regions[resolved.source.0];
        let join = logical_regions[resolved.join.0];
        if select == join || select == source || regions[select.0].instructions.len() != 1 {
            return Err(OperatingError::WrongOperation);
        }
        regions[select.0].ordered_existence = Some(OrderedExistence {
            source,
            number: resolved.shape.number,
            partition: resolved.shape.partition.into_boxed_slice(),
            order: resolved.shape.order,
            extremum: resolved.shape.extremum,
            output: resolved.output.into_boxed_slice(),
        });
        regions[source.0].consumers += 1;
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
    let observation_regions = super::observed::compile_unions(
        &mut regions,
        graph.observations.as_ref(),
        partitions,
        count,
    )?;
    // Resolve aliases into the same prepared dependency graph once. Runtime
    // completion consumes these edges without rescanning source kinds.
    let mut named_readers = vec![0usize; observation_regions.len()];
    for region in &mut regions {
        let mut dependencies = Vec::new();
        for source in region.inputs.iter() {
            match source {
                InputSource::Region(id) => dependencies.push(*id),
                InputSource::Observation(slot) => {
                    if let Some(Ok(root)) = observation_regions.get(*slot) {
                        dependencies.push(*root);
                        named_readers[*slot] += 1;
                    }
                }
                InputSource::Relation(_)
                | InputSource::Hosted { .. }
                | InputSource::ObservationMember { .. } => {}
            }
        }
        if let Some(ordered) = &region.ordered_existence {
            dependencies.push(ordered.source);
        }
        if let Some(column) = &region.column {
            dependencies.push(column.left());
        }
        region.dependencies = dependencies.into_boxed_slice();
    }
    for rule in &mut rules {
        let mut root_readers = rule.root_readers.to_vec();
        for source in rule
            .frontier
            .iter()
            .filter_map(|source| source.as_ref().ok())
        {
            if let InputSource::Observation(slot) = source {
                if let Some(count) = named_readers.get_mut(*slot) {
                    *count += 1;
                    if let Some(Ok(root)) = observation_regions.get(*slot) {
                        root_readers.push(*root);
                    }
                }
            }
        }
        rule.root_readers = root_readers.into_boxed_slice();
    }
    for (slot, root) in observation_regions.iter().enumerate() {
        if let Ok(root) = root {
            regions[root.0].consumers = named_readers[slot];
        }
    }
    for region in &mut regions {
        region.source_partitions = region
            .inputs
            .iter()
            .filter_map(|source| match source {
                InputSource::Relation(relation) => Some(SourcePartition {
                    source: *source,
                    requirement: partitions.relation(*relation),
                }),
                _ => None,
            })
            .collect();
    }
    let mut executable = ExecutableProgram {
        regions: regions.into_boxed_slice(),
        rules: rules.into_boxed_slice(),
        logical_regions: logical_regions.into_boxed_slice(),
        observation_regions: observation_regions.into_boxed_slice(),
    };
    for selection in super::selection::compile(&executable, prepared) {
        let target = selection.target();
        if !matches!(
            executable.regions[target.0].instructions[0].operation,
            Operation::Select { .. }
        ) {
            continue;
        }
        if executable.regions[target.0].selection.is_none() {
            executable.regions[target.0].selection = Some(selection);
        }
    }
    super::native::install(&mut executable, native_shapes);
    Ok(executable)
}

#[cfg(test)]
#[path = "observation_slot_test.rs"]
mod observation_slot_test;
