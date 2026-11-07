//! One part of the executable-program service: aliases of named outputs resolve into the same dependency graph once, each
//! region declares the partition of its relation inputs, and each region's lineage is found once, through the named observations
//! to the producer roots their members read.
use super::types::ProducerMember;
use super::types::region::value::Region;
use super::types::{
    InputSource, ObservationPlace, ProducerPartitions, ReaderMultiplicity, RegionId, Route,
    RuleProgram, SourcePartition,
};
use crate::domain::file_program::types::{GraphObservations, ResolvedObservationProducer};
use crate::domain::lineage::types::Lineage;
use crate::domain::operating::errors::OperatingError;
use ryo_values::domain::plan_parts::types::StepScope;

/// Runtime completion consumes these edges without rescanning source kinds.
pub(super) fn resolve(
    regions: &mut [Region],
    rules: &mut [RuleProgram],
    observation_regions: &[Result<RegionId, OperatingError>],
    partitions: &dyn ProducerPartitions,
) {
    let mut named_readers = vec![0usize; observation_regions.len()];
    for region in regions.iter_mut() {
        let mut dependencies = Vec::new();
        for source in region.inputs() {
            match source {
                InputSource::Region(id) => dependencies.push(*id),
                InputSource::Observation(ObservationPlace(slot)) => {
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
        match region.route() {
            Route::OrderedExistence(ordered) => dependencies.push(ordered.source),
            Route::Column(column) => dependencies.push(column.left),
            Route::Generic | Route::Selection(_) | Route::NativePositions(_) => {}
        }
        region.set_dependencies(dependencies.into_boxed_slice());
    }
    for rule in rules.iter_mut() {
        let mut root_readers = rule.root_readers.to_vec();
        for source in rule.frontier.iter().flatten() {
            if let InputSource::Observation(ObservationPlace(slot)) = source {
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
            regions[root.0].set_consumers(ReaderMultiplicity(named_readers[slot]));
        }
    }
    for region in regions.iter_mut() {
        let declared = region
            .inputs()
            .iter()
            .filter_map(|source| match source {
                InputSource::Relation(relation) => Some(SourcePartition {
                    source: *source,
                    requirement: partitions.relation(*relation),
                }),
                _ => None,
            })
            .collect();
        region.set_source_partitions(declared);
    }
}

/// What the lineage of a region's inputs is resolved against.
struct LineageSources<'a> {
    observation_regions: &'a [Result<RegionId, OperatingError>],
    named: Option<&'a GraphObservations>,
    observed_roots: &'a [Vec<Option<RegionId>>],
}

/// Sets the lineage of every region, after the aliases resolve.
pub(super) fn resolve_lineage(
    regions: &mut [Region],
    observation_regions: &[Result<RegionId, OperatingError>],
    named: Option<&GraphObservations>,
    observed_roots: &[Vec<Option<RegionId>>],
) {
    let sources = LineageSources {
        observation_regions,
        named,
        observed_roots,
    };
    let mut known: Vec<Option<Lineage>> = vec![None; regions.len()];
    for at in 0..regions.len() {
        let _lineage = region_lineage(RegionId(at), regions, &sources, &mut known);
    }
    for (region, lineage) in regions.iter_mut().zip(known) {
        region.set_lineage(lineage.unwrap_or_default());
    }
}

/// The lineage of one region, from its own instructions and its inputs' lineage, each region's found once.
fn region_lineage(
    id: RegionId,
    regions: &[Region],
    sources: &LineageSources<'_>,
    known: &mut [Option<Lineage>],
) -> Lineage {
    if let Some(Some(lineage)) = known.get(id.0) {
        return lineage.clone();
    }
    let Some(region) = regions.get(id.0) else {
        return Lineage::Mixed;
    };
    // A region read while its own lineage is being found proves nothing of it.
    known[id.0] = Some(Lineage::Mixed);
    let lineage = if region.phase() == StepScope::PerFile {
        // A per-file region reads the rows of one file only.
        match region
            .instructions()
            .last()
            .map(|instruction| instruction.contract().output_kinds.as_ref())
        {
            Some(Ok(kinds)) => crate::domain::lineage::service::one_file(kinds),
            Some(Err(_)) | None => Lineage::Mixed,
        }
    } else {
        let operands: Vec<Lineage> = region
            .inputs()
            .iter()
            .map(|source| source_lineage(*source, regions, sources, known))
            .collect();
        let mut lineage: Option<Lineage> = None;
        for instruction in region.instructions() {
            let Ok(kinds) = instruction.contract().output_kinds.as_ref() else {
                lineage = Some(Lineage::Mixed);
                break;
            };
            lineage = Some(match lineage {
                None => crate::domain::lineage::service::of_step(
                    instruction.operation(),
                    &operands,
                    kinds,
                ),
                // A continuation reads the row of the instruction before it.
                Some(previous) => crate::domain::lineage::service::of_step(
                    instruction.operation(),
                    std::slice::from_ref(&previous),
                    kinds,
                ),
            });
        }
        lineage.unwrap_or_default()
    };
    known[id.0] = Some(lineage.clone());
    lineage
}

/// The lineage of one input of a region.
fn source_lineage(
    source: InputSource,
    regions: &[Region],
    sources: &LineageSources<'_>,
    known: &mut [Option<Lineage>],
) -> Lineage {
    match source {
        InputSource::Relation(relation) => crate::domain::lineage::service::one_file(
            ryo_relations::domain::relations::service::kinds(relation),
        ),
        InputSource::Region(id) => region_lineage(id, regions, sources, known),
        InputSource::Observation(ObservationPlace(slot)) => {
            match sources.observation_regions.get(slot) {
                Some(Ok(root)) => region_lineage(*root, regions, sources, known),
                Some(Err(_)) | None => Lineage::Mixed,
            }
        }
        InputSource::ObservationMember { slot, member } => {
            match member_root(sources, slot, member) {
                Some(root) => region_lineage(root, regions, sources, known),
                None => Lineage::Mixed,
            }
        }
        InputSource::Hosted { .. } | InputSource::EmptyObservation(_) => Lineage::Mixed,
    }
}

/// The region of the producer output that member `member` of named slot `slot` reads.
fn member_root(
    sources: &LineageSources<'_>,
    slot: ObservationPlace,
    member: ProducerMember,
) -> Option<RegionId> {
    let named = sources.named?;
    let declaration = named.slots.get(slot.0)?;
    let producer = named
        .bindings
        .get(declaration.rule.0)?
        .get(declaration.input.0 as usize)?
        .input
        .producers
        .get(member.0)?;
    match producer {
        ResolvedObservationProducer::Output { rule, output } => {
            *sources.observed_roots.get(rule.0)?.get(output.0 as usize)?
        }
        ResolvedObservationProducer::NoQuery { .. } => None,
    }
}
