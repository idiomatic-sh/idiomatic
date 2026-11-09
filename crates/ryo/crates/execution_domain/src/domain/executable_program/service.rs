//! Lower the temporary canonical analysis graph; erase internal continuation cuts.
use crate::domain::source_partitions::types::{PartitionRequirement, ProducerPartitions};
#[cfg(any(test, feature = "fixtures"))]
use super::fusion::types::Ineligible;
use super::fusion::types::{Fusibility, RegionSharing};
use super::merge::merge_equivalent;
use super::types::demand_program::value::DemandProgram;
use super::types::executable_program::value::ExecutableProgram;
use super::types::instruction::value::Instruction;
use super::types::region::value::Region;
use super::types::*;
use crate::domain::operator::types::ReaderMultiplicity;
use crate::domain::{
    file_program::types::{NodeInput, RequestGraph},
    operating::errors::OperatingError,
};
use ryo_values::domain::plans::types::{Operation, Plan};

/// The analysis graph is consumed. Its native regions, layouts, cuts, liveness
/// tables and original-plan execution copies do not survive into this program.
pub fn compile(
    plans: &[Plan],
    graph: RequestGraph,
    partitions: &dyn ProducerPartitions,
    prepared: &crate::domain::file_program::types::RequestOperators,
    matcher: crate::domain::matching::types::PatternMatcher<'_>,
) -> Result<ExecutableProgram, OperatingError> {
    let count = graph.nodes.len();
    let mut origins = vec![None; count];
    for (rule, nodes) in graph.program_nodes.iter().enumerate() {
        for node in nodes.iter().flatten() {
            origins[node.0].get_or_insert(crate::domain::coverage::types::RuleIndex(rule));
        }
    }
    let ordered_alternatives = graph.ordered_existence.clone();
    let classification = super::fusion::service::classify(plans, &graph);
    let mut drafts: Vec<Option<DraftRegion<Instruction>>> = Vec::new();
    let mut mapping = vec![DraftIndex(usize::MAX); count];
    // A step's own schema fault, kept for its origin before the analysis graph is consumed.
    let faults: Vec<Option<OperatingError>> = graph
        .nodes
        .iter()
        .map(|node| node.contract.schema.as_ref().err().cloned())
        .collect();
    // Each node's presence and skip formula, folded once before the analysis graph is consumed.
    let presence = crate::domain::executable_program::presence::service::of_graph(
        &graph,
        &|relation| {
            if partitions.relation(relation) == PartitionRequirement::CurrentFile {
                crate::domain::executable_program::presence::types::RelationScope::CurrentFile
            } else {
                crate::domain::executable_program::presence::types::RelationScope::Shared
            }
        },
        matcher,
    );
    for (logical, node) in graph.nodes.into_iter().enumerate() {
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
        let mut actions =
            DemandProgram::admissions(node.contract.semantics, node.contract.inputs.used);
        if matches!(node.operation, Operation::Hosted { .. }) {
            actions.insert(0, DemandAction::Admit(InputId(0)));
        }
        let instruction = Instruction::new(
            LogicalInstruction(logical),
            rule,
            node.operation,
            node.contract,
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
            producer,
            draft.needs.map(Vec::into_boxed_slice),
            draft.phase,
            draft.instructions.into_boxed_slice(),
            inputs.into_boxed_slice(),
            readers[at],
        ));
    }
    // The ordered-existence replacement of a self-join anti-join select is the one implementation of
    // the regions the recogniser resolved; every other region is evaluated by its registered operators.
    let mut ordered: Vec<Option<OrderedExistence>> = vec![None; regions.len()];
    for alternative in ordered_alternatives.into_iter().flatten() {
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
        ordered[select.0] = Some(OrderedExistence {
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
        &regions,
        &faults,
    );
    #[cfg(any(test, feature = "fixtures"))]
    let synthesized = regions.len();
    let observation_regions = super::observed::compile_unions(
        &mut regions,
        graph.observations.as_ref(),
        partitions,
        LogicalInstruction(count),
    )?;
    // Named distinct-set unions are synthesized instructions: two members, never a continuation.
    // Only the controls read the table back, so only the test build keeps it.
    #[cfg(any(test, feature = "fixtures"))]
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
    for (at, shape) in ordered.into_iter().enumerate() {
        let route = shape.map_or(super::types::Route::Generic, |shape| {
            regions[shape.source.0].add_consumer();
            super::types::Route::OrderedExistence(shape)
        });
        regions[at].set_route(route);
    }
    super::aliases::resolve(&mut regions, &mut rules, &observation_regions, partitions);
    super::aliases::resolve_lineage(
        &mut regions,
        &observation_regions,
        graph.observations.as_ref(),
        &observed_roots,
    );
    // Merged twins compute the same rows from the same inputs, so they have the same formula.
    debug_assert!(
        representative
            .iter()
            .enumerate()
            .all(|(node, served)| { presence.nodes.get(node) == presence.nodes.get(served.0) })
    );
    // An ordered-existence route certifies its source's numbers from the relations the source's lineage read in this
    // file, so every region upstream of a route source runs whenever it is reached: an empty publication reads no relation.
    let mut certified = vec![false; regions.len()];
    let mut pending: Vec<RegionId> = regions
        .iter()
        .filter_map(|region| ordered_existence(region.route()).map(|shape| shape.source))
        .collect();
    while let Some(id) = pending.pop() {
        if let Some(seen) = certified.get_mut(id.0)
            && !*seen
        {
            *seen = true;
            pending.extend(
                regions
                    .get(id.0)
                    .map_or(&[][..], Region::dependencies)
                    .iter()
                    .copied(),
            );
        }
    }
    let presence = crate::domain::executable_program::presence::service::for_regions(
        presence,
        regions.iter().zip(&certified).map(|(region, certified)| {
            crate::domain::executable_program::presence::types::RegionNode {
                node: region
                    .instructions()
                    .last()
                    .map(|instruction| instruction.logical().0)
                    .filter(|logical| *logical < count)
                    .map(crate::domain::executable_program::presence::types::FoldedNode),
                certified: if *certified {
                    crate::domain::executable_program::presence::types::Certification::Certifying
                } else {
                    crate::domain::executable_program::presence::types::Certification::Free
                },
            }
        }),
    );
    let file_readers = super::file_demand::service::readers(&regions, &rules)?;
    let program = ExecutableProgram::new(
        regions.into_boxed_slice(),
        file_readers,
        rules.into_boxed_slice(),
        logical_regions.into_boxed_slice(),
        observation_regions.into_boxed_slice(),
    )
    .with_presence(presence);
    #[cfg(any(test, feature = "fixtures"))]
    let program = program.with_fusibility(fusibility.into_boxed_slice());
    Ok(program)
}

/// The ordered existence a route evaluates, when it is one.
pub fn ordered_existence(route: &Route) -> Option<&OrderedExistence> {
    match route {
        super::types::Route::OrderedExistence(shape) => Some(shape),
        _ => None,
    }
}
