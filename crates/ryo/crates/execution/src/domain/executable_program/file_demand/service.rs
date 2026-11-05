//! Compile immutable file-intent root obligations once for the whole request.
use super::super::types::{ExecutableProgram, InputSource, RegionId};
use crate::domain::{coverage::types::RuleIndex, operating::errors::OperatingError};
use ryo_values::domain::plans::types::StepScope;

use super::types::{FileMode, FileRoot, FileRuleDemand};

pub(crate) const fn mode(diagnostic: bool, observations: bool, capture: usize) -> usize {
    diagnostic as usize * 6 + observations as usize * 3 + capture
}
fn region(program: &ExecutableProgram, source: &InputSource) -> Option<RegionId> {
    match source {
        InputSource::Region(id) => Some(*id),
        InputSource::Observation(slot) => program
            .observation_regions()
            .get(*slot)?
            .as_ref()
            .ok()
            .copied(),
        _ => None,
    }
}
pub(crate) fn compile(
    program: &ExecutableProgram,
    rule: RuleIndex,
    observations: &[(Result<InputSource, OperatingError>, StepScope)],
    observation_frontiers: &[bool],
) -> Result<FileRuleDemand, OperatingError> {
    let prepared = program.rules().get(rule.0).ok_or(OperatingError::NotRun)?;
    let mut modes = Vec::with_capacity(12);
    for diagnostic in [false, true] {
        for named in [false, true] {
            for capture in 0..3 {
                let mut enabled = Vec::new();
                let mut roots = Vec::new();
                let mut named_outputs = Vec::new();
                if diagnostic {
                    enabled.extend(prepared.emit.iter().copied());
                    if let Ok(id) = prepared.emit {
                        if program.regions()[id.0].phase == StepScope::PerFile {
                            roots.push(FileRoot::Ordinary(id));
                        }
                    }
                }
                if named {
                    for (output, (source, phase)) in observations.iter().enumerate() {
                        if let Ok(InputSource::Region(id)) = source {
                            enabled.push(*id);
                        }
                        if *phase == StepScope::PerFile {
                            named_outputs.push(output);
                            if let Ok(source) = source {
                                if let Some(id) = region(program, source) {
                                    if program.regions()[id.0].phase == StepScope::PerFile {
                                        roots.push(FileRoot::Observation { region: id, output });
                                    }
                                }
                            }
                        }
                    }
                }
                for (at, source) in prepared.frontier.iter().enumerate() {
                    if capture == 0
                        || (capture == 1
                            && !observation_frontiers.get(at).copied().unwrap_or(false))
                    {
                        continue;
                    }
                    if let Ok(source) = source {
                        if let Some(id) = region(program, source) {
                            enabled.push(id);
                            if program.regions()[id.0].phase == StepScope::PerFile {
                                roots.push(FileRoot::Ordinary(id));
                            }
                        }
                    }
                }
                // Readers are a multiset: an Emit, observation and frontier may
                // name the same region but own distinct retirement obligations.
                let mut readers = Vec::new();
                for &root in &prepared.root_readers {
                    if let Some(at) = enabled.iter().position(|enabled| *enabled == root) {
                        enabled.swap_remove(at);
                        readers.push(root);
                    }
                }
                modes.push(FileMode {
                    readers: readers.into_boxed_slice(),
                    roots: roots.into_boxed_slice(),
                    observations: named_outputs.into_boxed_slice(),
                });
            }
        }
    }
    Ok(FileRuleDemand {
        outputs: observations.len(),
        modes: modes.into_boxed_slice(),
    })
}

/// Compute potential consumer obligations once, without opening or polling any input.
pub(crate) fn readers(
    regions: &[super::super::types::Region],
    rules: &[super::super::types::RuleProgram],
) -> Result<super::types::FileReaderGraph, OperatingError> {
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
        let mut consumers = HashMap::<RegionId, HashMap<RegionId, usize>>::new();
        while let Some(id) = pending.pop() {
            if !visited.insert(id) {
                continue;
            }
            let region = regions.get(id.0).ok_or(OperatingError::NotRun)?;
            for &dependency in &region.dependencies {
                *consumers
                    .entry(dependency)
                    .or_default()
                    .entry(id)
                    .or_default() += 1;
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
                    (producer, users.into_boxed_slice())
                })
                .collect(),
        );
    }
    Ok(super::types::FileReaderGraph {
        consumers: compiled,
    })
}

/// Count only selected external roots and their potentially live direct consumers.
/// Duplicate dependency edges remain distinct; shared consumers are counted once.
pub(crate) fn remaining(
    graph: &super::types::FileReaderGraph,
    selected: &std::collections::HashMap<RegionId, usize>,
    producer: RegionId,
) -> Result<usize, OperatingError> {
    let mut consumers = std::collections::HashMap::<RegionId, usize>::new();
    for root in selected.keys() {
        let roots = graph.consumers.get(root).ok_or(OperatingError::NotRun)?;
        if let Some(users) = roots.get(&producer) {
            for &(consumer, multiplicity) in users {
                consumers
                    .entry(consumer)
                    .and_modify(|count| *count = (*count).max(multiplicity))
                    .or_insert(multiplicity);
            }
        }
    }
    consumers.values().try_fold(
        selected.get(&producer).copied().unwrap_or(0),
        |count, used| {
            count
                .checked_add(*used)
                .ok_or(OperatingError::ShapeMismatch)
        },
    )
}
