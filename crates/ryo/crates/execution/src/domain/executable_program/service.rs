//! Lower the temporary canonical analysis graph; erase internal continuation cuts.
#[cfg(test)]
use super::fusion::types::Ineligible;
use super::fusion::types::{Fusibility, RegionSharing};
use super::merge::merge_equivalent;
use super::types::demand_program::value::DemandProgram;
use super::types::executable_program::value::ExecutableProgram;
use super::types::instruction::value::Instruction;
use super::types::region::value::Region;
use super::types::*;
use crate::domain::{
    file_program::types::{NodeInput, RequestGraph},
    operating::errors::OperatingError,
};
use ryo_values::domain::{
    plans::types::{Operation, Plan},
};

/// The analysis graph is consumed. Its native regions, layouts, cuts, liveness
/// tables and original-plan execution copies do not survive into this program.
pub(crate) fn compile(
    plans: &[Plan],
    graph: RequestGraph,
    partitions: &dyn ProducerPartitions,
    prepared: &crate::domain::operator::contract::types::RequestOperators,
    routes: RoutePolicy,
) -> Result<ExecutableProgram, OperatingError> {
    let mut column_candidates = super::column::ColumnCandidates::capture(&graph);
    let native_shapes = super::native::capture(&graph);
    let count = graph.nodes.len();
    let mut origins = vec![None; count];
    for (rule, nodes) in graph.program_nodes.iter().enumerate() {
        for node in nodes.iter().flatten() {
            origins[node.0].get_or_insert(crate::domain::coverage::types::RuleIndex(rule));
        }
    }
    let ordered_alternatives = graph.ordered_existence.clone();
    let classification = super::fusion::service::classify(plans, &graph, &mut column_candidates);
    let mut drafts: Vec<Option<DraftRegion<Instruction>>> = Vec::new();
    let mut mapping = vec![DraftIndex(usize::MAX); count];
    // A step's own schema fault, kept for its origin before the analysis graph is consumed.
    let faults: Vec<Option<OperatingError>> = graph
        .nodes
        .iter()
        .map(|node| node.contract.schema.as_ref().err().cloned())
        .collect();
    for (logical, mut node) in graph.nodes.into_iter().enumerate() {
        // PhysicalRegion's plans/layouts have no place in executable state.
        node.analysis.clear();
        let fuse = match (classification.fusibility[logical], node.inputs.first()) {
            (Fusibility::Fusable, Some(NodeInput::Node(child))) => {
                let DraftIndex(region) = mapping[child.0];
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
        let mut actions = DemandProgram::admissions(node.contract.semantics, node.contract.inputs.used);
        if matches!(node.operation, Operation::Hosted { .. }) {
            actions.insert(0, DemandAction::Admit(InputId(0)));
        }
        let instruction = Instruction::new(
            LogicalInstruction(logical),
            rule,
            node.operation,
            node.contract,
            if fuse.is_some() {
                vec![OperandSource::PreviousRow]
            } else {
                (0..node.inputs.len())
                    .map(|at| OperandSource::Boundary(InputId(at)))
                    .collect()
            }
            .into_boxed_slice(),
            DemandProgram::new(actions.into_boxed_slice()),
            partition,
        );
        let region = if let Some(region) = fuse {
            let region_body = drafts[region].as_mut().ok_or(OperatingError::NotRun)?;
            region_body.nodes.push(LogicalInstruction(logical));
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
                nodes: vec![LogicalInstruction(logical)],
                instructions: vec![instruction],
                inputs: node.inputs,
                needs: node.needs,
                sharing,
            }));
            region
        };
        mapping[logical] = DraftIndex(region);
    }
    // Equivalent regions of different rules compute once; their readers add up.
    let mut readers: Vec<ReaderMultiplicity> = drafts
        .iter()
        .map(|draft| {
            draft
                .as_ref()
                .and_then(|draft| draft.nodes.last())
                .map_or_else(ReaderMultiplicity::default, |last| {
                    classification.readers[last.0]
                })
        })
        .collect();
    // A merged node is served by its twin's instruction at the same place in the region.
    let mut representative: Vec<LogicalInstruction> = (0..count).map(LogicalInstruction).collect();
    merge_equivalent(&mut drafts, &mut mapping, &mut representative, &mut readers);
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
        .map(|DraftIndex(draft)| RegionId(dense[*draft]))
        .collect();
    let source = |input: NodeInput| -> InputSource {
        match input {
            NodeInput::Relation(relation) => InputSource::Relation(relation),
            NodeInput::Node(id) => InputSource::Region(logical_regions[id.0]),
            NodeInput::Observation(id) => InputSource::Observation(ObservationPlace(id.0)),
        }
    };
    let mut regions = Vec::with_capacity(next);
    for (at, draft) in drafts.into_iter().enumerate() {
        let Some(draft) = draft else { continue };
        let mut inputs: Vec<_> = draft.inputs.into_iter().map(source).collect();
        if let Some(first) = draft.instructions.first()
            && let Operation::Hosted { relation } = first.operation()
        {
            inputs.push(InputSource::Hosted {
                rule: first.producer(),
                relation: *relation,
            });
        }
        let producer = draft
            .instructions
            .last()
            .ok_or(OperatingError::NotRun)?
            .producer();
        regions.push(Region::new(
            RegionId(dense[at]),
            producer,
            draft.needs.map(Vec::into_boxed_slice),
            draft.phase,
            draft.instructions.into_boxed_slice(),
            inputs.into_boxed_slice(),
            readers[at],
        ));
    }
    // Every family leaves what it found eligible here; one rule keeps one route per region below.
    let mut candidates: Vec<super::types::RouteOffers> =
        vec![super::types::RouteOffers::default(); regions.len()];
    for column in column_candidates
        .lower(&logical_regions)
        .into_iter()
        .filter(|_| routes.column_kernel == RouteWiring::Wired)
    {
        let target = column.target;
        if candidates[target.0].column.is_some() {
            continue;
        }
        candidates[target.0].column = Some(column);
    }
    for alternative in ordered_alternatives
        .into_iter()
        .flatten()
        .filter(|_| routes.ordered_existence == RouteWiring::Wired)
    {
        let crate::domain::file_program::types::ResolvedAlternative::Eligible(resolved) =
            alternative
        else {
            continue;
        };
        let select = logical_regions[resolved.select.0];
        let source = logical_regions[resolved.source.0];
        let join = logical_regions[resolved.join.0];
        if select == join || select == source || regions[select.0].instructions().len() != 1 {
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
    let RulePrograms {
        mut rules,
        observed_roots,
    } = super::origins::rule_programs(
        plans,
        graph.program_nodes,
        graph.observations.as_ref(),
        &representative,
        &logical_regions,
        &faults,
    );
    #[cfg(test)]
    let synthesized = regions.len();
    let observation_regions = super::observed::compile_unions(
        &mut regions,
        graph.observations.as_ref(),
        partitions,
        LogicalInstruction(count),
    )?;
    // Named distinct-set unions are synthesized instructions: two members, never a continuation.
    // Only the controls read the table back, so only the test build keeps it.
    #[cfg(test)]
    let fusibility = {
        let mut fusibility = classification.fusibility;
        for instruction in regions[synthesized..]
            .iter()
            .flat_map(|region| region.instructions().iter())
        {
            if fusibility.len() <= instruction.logical().0 {
                fusibility.resize(
                    instruction.logical().0 + 1,
                    Fusibility::Excluded(Ineligible::MultiInput),
                );
            }
        }
        fusibility
    };
    // The stored-selection and native-position families are offered over the same regions, then
    // one rule keeps one route per region. Only the kept route takes a reader of its source.
    candidates.resize_with(regions.len(), Default::default);
    for selection in super::selection::compile(&rules, &logical_regions, &regions, prepared)
        .into_iter()
        .filter(|_| routes.stored_selection == RouteWiring::Wired)
    {
        let target = selection.target;
        if !matches!(
            regions[target.0].instructions()[0].operation(),
            Operation::Select { .. } | Operation::Project { .. }
        ) {
            continue;
        }
        if candidates[target.0].selection.is_none() {
            candidates[target.0].selection = Some(selection);
        }
    }
    if routes.native_positions == RouteWiring::Wired {
        super::native::offer(&regions, &native_shapes, &mut candidates);
    }
    // Native positions are published for a reader that consumes positions. A region read by one
    // that has no native route must hand it rows, so it keeps no native offer: a stored selection
    // when one was offered, the generic route otherwise. A named observation is read by
    // synthesized unions and the consumers of its members, none of which has a native route, so
    // the region each declared output reads is read by rows.
    let mut read_by_rows = vec![false; regions.len()];
    for (reader, region) in regions.iter().enumerate() {
        if candidates[reader].native.is_some() {
            continue;
        }
        for source in region.inputs() {
            if let InputSource::Region(id) = source {
                read_by_rows[id.0] = true;
            }
        }
    }
    for root in observed_roots.iter().flatten().flatten() {
        read_by_rows[root.0] = true;
    }
    for (at, candidate) in candidates.iter_mut().enumerate() {
        if read_by_rows[at] {
            candidate.native = None;
        }
    }
    for (at, candidate) in candidates.into_iter().enumerate() {
        let route = choose(candidate);
        match &route {
            super::types::Route::OrderedExistence(shape) => regions[shape.source.0].add_consumer(),
            super::types::Route::Column(shape) => regions[shape.left.0].add_consumer(),
            _ => {}
        }
        regions[at].set_route(route);
    }
    super::aliases::resolve(&mut regions, &mut rules, &observation_regions, partitions);
    super::aliases::resolve_lineage(
        &mut regions,
        &observation_regions,
        graph.observations.as_ref(),
        &observed_roots,
    );
    let file_readers = super::file_demand::service::readers(&regions, &rules)?;
    let program = ExecutableProgram::new(
        regions.into_boxed_slice(),
        file_readers,
        rules.into_boxed_slice(),
        logical_regions.into_boxed_slice(),
        observation_regions.into_boxed_slice(),
        routes,
    );
    #[cfg(test)]
    let program = program.with_fusibility(fusibility.into_boxed_slice());
    Ok(program)
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
pub(crate) fn column(route: &super::types::Route) -> Option<&super::types::ColumnAlternative> {
    match route {
        super::types::Route::Column(shape) => Some(shape),
        _ => None,
    }
}
pub(crate) fn selection(
    route: &super::types::Route,
) -> Option<&super::types::SelectionAlternative> {
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
