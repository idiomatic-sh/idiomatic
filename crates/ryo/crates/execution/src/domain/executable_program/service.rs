//! Lower the temporary canonical analysis graph; erase internal continuation cuts.
#[cfg(test)]
use super::fusion::types::Ineligible;
use super::fusion::types::{Fusibility, RegionSharing};
use super::types::*;
use crate::domain::{
    file_program::types::{NodeId, NodeInput, RequestGraph},
    operating::errors::OperatingError,
    operator::contract::types::{DemandOrder, SemanticContract},
};
use ryo_values::domain::{
    plan_parts::types::{Input, StepId},
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
    /// Whether every instruction's answer depends only on its operation and inputs.
    sharing: RegionSharing,
}
/// The canonical inputs of a region: producers named by the region that now serves them.
fn canonical(draft: &DraftRegion, mapping: &[usize]) -> Vec<NodeInput> {
    draft
        .inputs
        .iter()
        .map(|input| match input {
            NodeInput::Node(child) => NodeInput::Node(NodeId(mapping[child.0])),
            other => *other,
        })
        .collect()
}
/// Equal regions of different rules compute once: same instructions, same inputs, same bindings.
fn equivalent(left: &DraftRegion, right: &DraftRegion, mapping: &[usize]) -> bool {
    left.phase == right.phase
        && left.instructions.len() == right.instructions.len()
        && left
            .instructions
            .iter()
            .zip(&right.instructions)
            .all(|(a, b)| a.operation == b.operation && a.partition == b.partition)
        && canonical(left, mapping) == canonical(right, mapping)
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
    let mut origins = vec![None; count];
    for (rule, nodes) in graph.program_nodes.iter().enumerate() {
        for node in nodes.iter().filter_map(|node| node.as_ref().ok()) {
            origins[node.0].get_or_insert(crate::domain::coverage::types::RuleIndex(rule));
        }
    }
    let ordered_alternatives = graph.ordered_existence.clone();
    let classification = super::fusion::service::classify(plans, &graph, &mut column_candidates);
    let mut drafts: Vec<Option<DraftRegion>> = Vec::new();
    let mut mapping = vec![usize::MAX; count];
    for (logical, mut node) in graph.nodes.into_iter().enumerate() {
        // PhysicalRegion's plans/layouts have no place in executable state.
        node.analysis.clear();
        let fuse = match (classification.fusibility[logical], node.inputs.first()) {
            (Fusibility::Fusable, Some(NodeInput::Node(child))) => {
                let region = mapping[child.0];
                if drafts.get(region).is_none_or(Option::is_none) {
                    return Err(OperatingError::NotRun);
                }
                Some(region)
            }
            _ => None,
        };
        let sharing = super::fusion::service::region_sharing(&node, classification.held[logical]);
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
            if sharing == RegionSharing::RuleScoped {
                region_body.sharing = RegionSharing::RuleScoped;
            }
            region
        } else {
            let region = drafts.len();
            drafts.push(Some(DraftRegion {
                phase: node.phase,
                nodes: vec![logical],
                instructions: vec![instruction],
                inputs: node.inputs,
                needs: node.needs,
                sharing,
            }));
            region
        };
        mapping[logical] = region;
    }
    // Equivalent regions of different rules compute once; their readers add up.
    let mut readers: Vec<ReaderMultiplicity> = drafts
        .iter()
        .map(|draft| {
            draft
                .as_ref()
                .and_then(|draft| draft.nodes.last())
                .map_or_else(ReaderMultiplicity::default, |last| {
                    classification.readers[*last]
                })
        })
        .collect();
    // A merged node is served by its twin's instruction at the same place in the region.
    let mut representative: Vec<usize> = (0..count).collect();
    let mut shared: Vec<usize> = Vec::new();
    for at in 0..drafts.len() {
        let Some(draft) = drafts[at].as_ref() else {
            continue;
        };
        if draft.sharing == RegionSharing::RuleScoped {
            continue;
        }
        let twin = shared.iter().copied().find(|earlier| {
            drafts[*earlier]
                .as_ref()
                .is_some_and(|earlier| equivalent(earlier, draft, &mapping))
        });
        let Some(twin) = twin else {
            shared.push(at);
            continue;
        };
        let Some(merged) = drafts[at].take() else {
            continue;
        };
        if let Some(kept) = drafts[twin].as_ref() {
            for (node, served) in merged.nodes.iter().zip(&kept.nodes) {
                representative[*node] = *served;
            }
        }
        for node in &merged.nodes {
            mapping[*node] = twin;
        }
        for input in &merged.inputs {
            if let NodeInput::Node(child) = input {
                let producer = mapping[child.0];
                readers[producer].0 = readers[producer].0.saturating_sub(1);
            }
        }
        readers[twin].0 += readers[at].0;
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
            consumers: readers[at].0,
            dependencies: Box::new([]),
            route: super::types::Route::Generic,
        });
    }
    // Every family leaves what it found eligible here; one rule keeps one route per region below.
    let mut candidates: Vec<super::types::RouteOffers> =
        vec![super::types::RouteOffers::default(); regions.len()];
    for column in column_candidates.lower(&logical_regions) {
        let target = column.target();
        if candidates[target.0].column.is_some() {
            continue;
        }
        candidates[target.0].column = Some(column);
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
        candidates[select.0].ordered = Some(OrderedExistence {
            source,
            number: resolved.shape.number,
            partition: resolved.shape.partition.into_boxed_slice(),
            order: resolved.shape.order,
            extremum: resolved.shape.extremum,
            output: resolved.output.into_boxed_slice(),
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
                    logical: LogicalInstruction(representative[id.0]),
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
    #[cfg(test)]
    let synthesized = regions.len();
    let observation_regions = super::observed::compile_unions(
        &mut regions,
        graph.observations.as_ref(),
        partitions,
        count,
    )?;
    // Named distinct-set unions are synthesized instructions: two members, never a continuation.
    // Only the controls read the table back, so only the test build keeps it.
    #[cfg(test)]
    let fusibility = {
        let mut fusibility = classification.fusibility;
        for instruction in regions[synthesized..]
            .iter()
            .flat_map(|region| region.instructions.iter())
        {
            if fusibility.len() <= instruction.logical.0 {
                fusibility.resize(
                    instruction.logical.0 + 1,
                    Fusibility::Excluded(Ineligible::MultiInput),
                );
            }
        }
        fusibility
    };
    // The stored-selection and native-position families are offered over the same regions, then
    // one rule keeps one route per region. Only the kept route takes a reader of its source.
    candidates.resize_with(regions.len(), Default::default);
    for selection in super::selection::compile(&rules, &logical_regions, &regions, prepared) {
        let target = selection.target();
        if !matches!(
            regions[target.0].instructions[0].operation,
            Operation::Select { .. } | Operation::Project { .. }
        ) {
            continue;
        }
        if candidates[target.0].selection.is_none() {
            candidates[target.0].selection = Some(selection);
        }
    }
    super::native::offer(&regions, &native_shapes, &mut candidates);
    for (at, candidate) in candidates.into_iter().enumerate() {
        let route = choose(candidate);
        match &route {
            super::types::Route::OrderedExistence(shape) => regions[shape.source.0].consumers += 1,
            super::types::Route::Column(shape) => regions[shape.left().0].consumers += 1,
            _ => {}
        }
        regions[at].route = route;
    }
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
                | InputSource::ObservationMember { .. }
                | InputSource::EmptyObservation(_) => {}
            }
        }
        match &region.route {
            super::types::Route::OrderedExistence(ordered) => dependencies.push(ordered.source),
            super::types::Route::Column(column) => dependencies.push(column.left()),
            super::types::Route::Generic
            | super::types::Route::Selection(_)
            | super::types::Route::NativePositions(_) => {}
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
    let file_readers = super::file_demand::service::readers(&regions, &rules)?;
    Ok(ExecutableProgram {
        file_readers,
        regions: regions.into_boxed_slice(),
        rules: rules.into_boxed_slice(),
        logical_regions: logical_regions.into_boxed_slice(),
        observation_regions: observation_regions.into_boxed_slice(),
        #[cfg(test)]
        fusibility: fusibility.into_boxed_slice(),
    })
}

/// The one selection rule: ordered existence, then the column kernel, then native positions
/// (a native join consumes the positions of its inputs, so they must be published as such),
/// then the stored selection; the generic route when no family offered one. A route
/// that cannot run declines before any observable work and the generic evaluation of the
/// instruction then runs once; no second specialised route is ever tried.
pub(crate) fn choose(offers: super::types::RouteOffers) -> super::types::Route {
    use super::types::Route;
    if let Some(shape) = offers.ordered {
        Route::OrderedExistence(shape)
    } else if let Some(shape) = offers.column {
        Route::Column(shape)
    } else if let Some(shape) = offers.native {
        Route::NativePositions(shape)
    } else if let Some(shape) = offers.selection {
        Route::Selection(shape)
    } else {
        Route::Generic
    }
}
pub(crate) fn ordered_existence(route: &super::types::Route) -> Option<&OrderedExistence> {
    match route {
        super::types::Route::OrderedExistence(shape) => Some(shape),
        _ => None,
    }
}
pub(crate) fn column(route: &super::types::Route) -> Option<&super::column::ColumnAlternative> {
    match route {
        super::types::Route::Column(shape) => Some(shape),
        _ => None,
    }
}
pub(crate) fn selection(
    route: &super::types::Route,
) -> Option<&super::selection::SelectionAlternative> {
    match route {
        super::types::Route::Selection(shape) => Some(shape),
        _ => None,
    }
}
pub(crate) fn native_positions(
    route: &super::types::Route,
) -> Option<&std::sync::Arc<crate::domain::lowering::types::LoweredShape>> {
    match route {
        super::types::Route::NativePositions(shape) => Some(shape),
        _ => None,
    }
}
