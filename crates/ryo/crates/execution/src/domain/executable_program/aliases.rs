//! One part of the executable-program service: aliases of named outputs resolve into the same dependency graph once, and
//! each region declares the partition of its relation inputs.
use super::types::region::value::Region;
use super::types::{
    InputSource, ObservationPlace, ProducerPartitions, ReaderMultiplicity, RegionId, Route,
    RuleProgram, SourcePartition,
};
use crate::domain::operating::errors::OperatingError;

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
            Route::Generic => {}
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
