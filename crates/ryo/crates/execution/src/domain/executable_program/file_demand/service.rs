//! Compile immutable file-intent root obligations once for the whole request.
use super::super::types::executable_program::value::ExecutableProgram;
use super::super::types::region::value::Region;
use super::super::types::{
    ComputationCount, ConsumerObligation, FileReaderGraph, InputSource, ObservationPlace,
    ReaderMultiplicity, RegionId, Remaining, RemainingMemo, RemainingTable, RuleProgram,
};
use super::types::*;
use crate::domain::file_program::types::FrontierPurpose;
use crate::domain::{
    coverage::types::RuleIndex, operating::errors::OperatingError,
};
use ryo_values::domain::{
    plan_parts::types::StepScope,
    records::types::ObservationIndex,
};

fn region(
    program: &ExecutableProgram,
    source: &InputSource,
) -> Result<Option<RegionId>, OperatingError> {
    match source {
        InputSource::Region(id) => Ok(Some(*id)),
        InputSource::Observation(ObservationPlace(slot)) => program
            .observation_regions()
            .get(*slot)
            .ok_or(OperatingError::NotRun)?
            .clone()
            .map(Some),
        _ => Ok(None),
    }
}
/// Root candidates retain deferred source faults without opening a dormant input.
fn file_root(
    program: &ExecutableProgram,
    source: Result<Option<RegionId>, OperatingError>,
    roots: &mut Vec<FileRootDemand>,
    output: Option<ObservationIndex>,
) {
    let id = match source {
        Ok(Some(id)) => id,
        Ok(None) => return,
        Err(error) => {
            roots.push(FileRootDemand::Deferred(error));
            return;
        }
    };
    if program.regions()[id.0].phase() != StepScope::PerFile {
        return;
    }
    roots.push(match output {
        Some(output) => FileRootDemand::Observation { region: id, output },
        None => FileRootDemand::Ordinary(id),
    });
}
fn compile_mode(
    program: &ExecutableProgram,
    prepared: &RuleProgram,
    observations: &[ObservationDemandSource],
    frontiers: &[FrontierPurpose],
    demand: FileDemand,
) -> Result<FileMode, OperatingError> {
    let mut enabled = Vec::new();
    let mut roots = Vec::new();
    let mut named_outputs = Vec::new();
    if demand.diagnostic == RootSelection::Required {
        enabled.extend(prepared.emit.iter().copied());
        file_root(program, prepared.emit.clone().map(Some), &mut roots, None);
    }
    if demand.observations == RootSelection::Required {
        for (output, observation) in observations.iter().enumerate() {
            if let Ok(InputSource::Region(id)) = &observation.source {
                enabled.push(*id);
            }
            if observation.phase != StepScope::PerFile {
                continue;
            }
            let output = ObservationIndex(
                output
                    .try_into()
                    .map_err(|_| OperatingError::ShapeMismatch)?,
            );
            named_outputs.push(output);
            let source = observation
                .source
                .as_ref()
                .map_err(Clone::clone)
                .and_then(|source| region(program, source));
            file_root(program, source, &mut roots, Some(output));
        }
    }
    for (at, source) in prepared.frontier.iter().enumerate() {
        match demand.capture {
            FrontierSelection::Disabled => continue,
            FrontierSelection::Observations
                if frontiers.get(at) != Some(&FrontierPurpose::Observation) =>
            {
                continue;
            }
            _ => {}
        }
        let source = source
            .as_ref()
            .map_err(Clone::clone)
            .and_then(|source| region(program, source));
        if let Ok(Some(id)) = &source {
            enabled.push(*id);
        }
        file_root(program, source, &mut roots, None);
    }
    // Reader multiplicity is independent of scheduling deduplication.
    let mut readers = Vec::new();
    for &root in &prepared.root_readers {
        if let Some(at) = enabled.iter().position(|enabled| *enabled == root) {
            enabled.swap_remove(at);
            readers.push(root);
        }
    }
    Ok(FileMode {
        readers: readers.into_boxed_slice(),
        roots: roots.into_boxed_slice(),
        observations: named_outputs.into_boxed_slice(),
    })
}
pub(crate) fn compile(
    program: &ExecutableProgram,
    rule: RuleIndex,
    observations: &[ObservationDemandSource],
    frontiers: &[FrontierPurpose],
) -> Result<FileRuleDemand, OperatingError> {
    let prepared = program.rules().get(rule.0).ok_or(OperatingError::NotRun)?;
    let demands = [RootSelection::Undemanded, RootSelection::Required]
        .into_iter()
        .flat_map(|diagnostic| {
            [RootSelection::Undemanded, RootSelection::Required]
                .into_iter()
                .flat_map(move |observations| {
                    [
                        FrontierSelection::Disabled,
                        FrontierSelection::Observations,
                        FrontierSelection::All,
                    ]
                    .into_iter()
                    .map(move |capture| FileDemand {
                        diagnostic,
                        observations,
                        capture,
                    })
                })
        });
    let modes = demands
        .map(|demand| compile_mode(program, prepared, observations, frontiers, demand))
        .collect::<Result<Vec<_>, _>>()?
        .into_boxed_slice();
    Ok(FileRuleDemand {
        outputs: NamedOutputCount(observations.len()),
        modes,
    })
}

/// Compute potential consumer obligations once, without opening or polling any input.
pub(crate) fn readers(
    regions: &[Region],
    rules: &[RuleProgram],
) -> Result<FileReaderGraph, OperatingError> {
    use std::collections::{HashMap, HashSet};
    let mut compiled = HashMap::new();
    for root in rules
        .iter()
        .flat_map(|rule| rule.root_readers.iter().copied())
    {
        if compiled.contains_key(&root) {
            continue;
        }
        let mut pending = vec![root];
        let mut visited = HashSet::new();
        let mut consumers = HashMap::<RegionId, HashMap<RegionId, ReaderMultiplicity>>::new();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            let region = regions.get(id.0).ok_or(OperatingError::NotRun)?;
            for &dependency in region.dependencies() {
                consumers
                    .entry(dependency)
                    .or_default()
                    .entry(id)
                    .or_default()
                    .0 += 1;
                pending.push(dependency);
            }
        }
        compiled.insert(
            root,
            consumers
                .into_iter()
                .map(|(producer, users)| {
                    let mut users = users.into_iter().collect::<Vec<_>>();
                    users.sort_unstable_by_key(|(id, _)| id.0);
                    (
                        producer,
                        users
                            .into_iter()
                            .map(|(consumer, readers)| ConsumerObligation { consumer, readers })
                            .collect::<Vec<_>>()
                            .into_boxed_slice(),
                    )
                })
                .collect(),
        );
    }
    Ok(FileReaderGraph {
        consumers: compiled,
        remaining: Default::default(),
    })
}

/// Initial reader counts for the selected roots, computed once and shared by every
/// later file that selects the same roots.
pub(crate) fn remaining(
    graph: &FileReaderGraph,
    regions: &[Region],
    selected: &std::collections::HashMap<RegionId, ReaderMultiplicity>,
) -> Result<Remaining, OperatingError> {
    let mut memo = graph
        .remaining
        .lock()
        .map_err(|_| OperatingError::NotRun)?;
    if let Some(table) = &memo.table
        && table.selected == *selected
    {
        return Ok(Remaining {
            table: std::sync::Arc::clone(table),
            #[cfg(test)]
            computations: memo.computations,
        });
    }
    let table = std::sync::Arc::new(table(graph, regions, selected)?);
    memo.computations = ComputationCount(memo.computations.0 + 1);
    memo.table = Some(std::sync::Arc::clone(&table));
    Ok(Remaining {
        table,
        #[cfg(test)]
        computations: memo.computations,
    })
}

/// Count only selected external roots and their potentially live direct consumers.
/// Duplicate dependency edges remain distinct; shared consumers are counted once.
fn table(
    graph: &FileReaderGraph,
    regions: &[Region],
    selected: &std::collections::HashMap<RegionId, ReaderMultiplicity>,
) -> Result<RemainingTable, OperatingError> {
    let mut consumers =
        vec![std::collections::HashMap::<RegionId, ReaderMultiplicity>::new(); regions.len()];
    for root in selected.keys() {
        let producers = graph.consumers.get(root).ok_or(OperatingError::NotRun)?;
        for (producer, users) in producers {
            let slot = consumers
                .get_mut(producer.0)
                .ok_or(OperatingError::NotRun)?;
            for user in users {
                slot.entry(user.consumer)
                    .and_modify(|count| *count = ReaderMultiplicity(count.0.max(user.readers.0)))
                    .or_insert(user.readers);
            }
        }
    }
    let counts = consumers
        .iter()
        .enumerate()
        .map(|(at, users)| {
            users.values().try_fold(
                selected.get(&RegionId(at)).copied().unwrap_or_default(),
                |count, used| {
                    count
                        .0
                        .checked_add(used.0)
                        .map(ReaderMultiplicity)
                        .ok_or(OperatingError::ShapeMismatch)
                },
            )
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_boxed_slice();
    Ok(RemainingTable {
        selected: selected.clone(),
        counts,
    })
}

/// The initial reader count of `producer` in a table: its dense count, or the selected root's own.
pub(crate) fn reader_count(table: &RemainingTable, producer: RegionId) -> ReaderMultiplicity {
    table
        .counts
        .get(producer.0)
        .copied()
        .unwrap_or_else(|| table.selected.get(&producer).copied().unwrap_or_default())
}
