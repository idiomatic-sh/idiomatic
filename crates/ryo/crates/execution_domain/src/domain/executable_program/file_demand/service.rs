//! Compile immutable file-intent root obligations once for the whole request.
use super::super::types::executable_program::value::ExecutableProgram;
use super::super::types::region::value::Region;
use super::super::types::{
    FileReaderGraph, InputSource, ObservationPlace, RegionId, RemainingTable, RuleProgram,
};
use super::types::*;
use crate::domain::executable_program::types::absent_roots::value::AbsentRoots;
use crate::domain::executable_program::types::file_rule_demand::value::FileRuleDemand;
use crate::domain::operator::types::ReaderMultiplicity;
use crate::domain::rule_intent::types::FrontierPurpose;
use crate::domain::{coverage::types::RuleIndex, operating::errors::OperatingError};
use ryo_values::domain::{plan_parts::types::StepScope, records::types::ObservationIndex};

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
/// Compiles a rule's file demand in every demand mode from the program, its observations and its frontiers.
pub fn compile(
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
    Ok(FileRuleDemand::compiled(
        NamedOutputCount(observations.len()),
        modes,
    ))
}

/// The regions a file may select as external readers, collected once for the request.
pub(crate) fn readers(
    regions: &[Region],
    rules: &[RuleProgram],
) -> Result<FileReaderGraph, OperatingError> {
    let mut roots = std::collections::HashSet::new();
    for root in rules
        .iter()
        .flat_map(|rule| rule.root_readers.iter().copied())
    {
        let _region = regions.get(root.0).ok_or(OperatingError::NotRun)?;
        let _new = roots.insert(root);
    }
    Ok(FileReaderGraph { roots })
}

/// One file's initial reader counts, by one walk from its admitted roots. Each selected root counts its own
/// multiplicity, absent or not. Each region reached from an admitted root without passing through an absent root adds one
/// to each of its dependencies, once per declared edge, so duplicate edges stay distinct and a shared consumer is counted
/// once. An absent root is reached and counted, but its own edges are never followed: nothing only it reaches gets a
/// count, at any depth.
pub fn table(
    regions: &[Region],
    selected: &[SelectedRoot],
    absent: &AbsentRoots,
) -> Result<RemainingTable, OperatingError> {
    let mut counts = vec![ReaderMultiplicity::default(); regions.len()].into_boxed_slice();
    // A region is pushed once, when it is first reached, so the stack never outgrows the program.
    let mut queued = vec![false; regions.len()];
    let mut pending = Vec::with_capacity(regions.len());
    for &SelectedRoot {
        region: root,
        readers,
    } in selected
    {
        let count = counts.get_mut(root.0).ok_or(OperatingError::NotRun)?;
        count.0 = count
            .0
            .checked_add(readers.0)
            .ok_or(OperatingError::ShapeMismatch)?;
        let seen = queued.get_mut(root.0).ok_or(OperatingError::NotRun)?;
        if !*seen
            && absent.presence(root)
                == crate::domain::executable_program::presence::types::RegionPresence::Present
        {
            *seen = true;
            pending.push(root);
        }
    }
    while let Some(id) = pending.pop() {
        if absent.presence(id)
            == crate::domain::executable_program::presence::types::RegionPresence::Absent
        {
            continue;
        }
        let region = regions.get(id.0).ok_or(OperatingError::NotRun)?;
        for &dependency in region.dependencies() {
            let count = counts.get_mut(dependency.0).ok_or(OperatingError::NotRun)?;
            count.0 = count
                .0
                .checked_add(1)
                .ok_or(OperatingError::ShapeMismatch)?;
            let seen = queued.get_mut(dependency.0).ok_or(OperatingError::NotRun)?;
            if !*seen {
                *seen = true;
                pending.push(dependency);
            }
        }
    }
    Ok(RemainingTable { counts })
}

/// The initial reader count of `producer` in a table, which counts every region of its program.
///
/// # Errors
///
/// [`OperatingError::NotRun`] for a region the program does not have.
pub fn reader_count(
    table: &RemainingTable,
    producer: RegionId,
) -> Result<ReaderMultiplicity, OperatingError> {
    table
        .counts
        .get(producer.0)
        .copied()
        .ok_or(OperatingError::NotRun)
}
