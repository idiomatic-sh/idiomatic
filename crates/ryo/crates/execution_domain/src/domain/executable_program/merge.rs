//! One part of the executable-program service: equivalent drafted regions of different rules compute once.
use super::fusion::types::RegionSharing;
use super::types::instruction::value::Instruction;
use super::types::{
    ComparisonCount, DraftIndex, DraftRegion, Equivalence, LogicalInstruction, RegionBucket,
    RegionMerge,
};
use crate::domain::file_program::types::{NodeInput, ProgramNodeId};
use crate::domain::operator::types::ReaderMultiplicity;

/// The canonical inputs of a region: producers named by the region that now serves them.
/// Built once per region; every later comparison and lookup reads this form.
fn canonical(draft: &DraftRegion<Instruction>, mapping: &[DraftIndex]) -> Vec<NodeInput> {
    draft
        .inputs
        .iter()
        .map(|input| match input {
            NodeInput::Node(child) => NodeInput::Node(ProgramNodeId(mapping[child.0].0)),
            other => *other,
        })
        .collect()
}
/// Equal regions of different rules compute once: same instructions, same inputs, same bindings.
/// Compares only; the canonical inputs of both sides are already built.
fn equivalent(
    left: &DraftRegion<Instruction>,
    left_inputs: &[NodeInput],
    right: &DraftRegion<Instruction>,
    right_inputs: &[NodeInput],
) -> Equivalence {
    let same = left.phase == right.phase
        && left.instructions.len() == right.instructions.len()
        && left
            .instructions
            .iter()
            .zip(&right.instructions)
            .all(|(a, b)| a.operation() == b.operation() && a.partition() == b.partition())
        && left_inputs == right_inputs;
    if same {
        Equivalence::Same
    } else {
        Equivalence::Different
    }
}
/// The bucket of a region: a hash of everything `equivalent` compares except the phase.
/// Equivalent regions always share a bucket; a collision is settled by `equivalent`.
fn bucket(draft: &DraftRegion<Instruction>, canonical: &[NodeInput]) -> RegionBucket {
    use std::hash::{Hash, Hasher};
    let mut state = std::hash::DefaultHasher::new();
    canonical.hash(&mut state);
    draft.instructions.len().hash(&mut state);
    for instruction in &draft.instructions {
        instruction.operation().hash(&mut state);
        instruction.partition().hash(&mut state);
    }
    RegionBucket(state.finish())
}
/// Equivalent regions of different rules compute once; their readers add up.
/// The first equivalent earlier region serves the later one.
pub(super) fn merge_equivalent(
    drafts: &mut [Option<DraftRegion<Instruction>>],
    mapping: &mut [DraftIndex],
    representative: &mut [LogicalInstruction],
    readers: &mut [ReaderMultiplicity],
) -> RegionMerge {
    let mut merge = RegionMerge {
        shared: Vec::new(),
        comparisons: ComparisonCount(0),
    };
    // Canonical inputs of each kept region, built when it is kept; its children are
    // final by then because every child region precedes it.
    let mut kept_inputs: Vec<Vec<NodeInput>> = vec![Vec::new(); drafts.len()];
    let mut buckets: std::collections::HashMap<RegionBucket, Vec<usize>> =
        std::collections::HashMap::new();
    for at in 0..drafts.len() {
        let Some(draft) = drafts[at].as_ref() else {
            continue;
        };
        if draft.sharing == RegionSharing::RuleScoped {
            continue;
        }
        let inputs = canonical(draft, mapping);
        let key = bucket(draft, &inputs);
        let twin = buckets.get(&key).and_then(|candidates| {
            candidates.iter().copied().find(|earlier| {
                merge.comparisons = ComparisonCount(merge.comparisons.0 + 1);
                drafts[*earlier].as_ref().is_some_and(|kept| {
                    equivalent(kept, &kept_inputs[*earlier], draft, &inputs) == Equivalence::Same
                })
            })
        });
        let Some(twin) = twin else {
            merge.shared.push(DraftIndex(at));
            buckets.entry(key).or_default().push(at);
            kept_inputs[at] = inputs;
            continue;
        };
        let Some(merged) = drafts[at].take() else {
            continue;
        };
        if let Some(kept) = drafts[twin].as_ref() {
            for (node, served) in merged.nodes.iter().zip(&kept.nodes) {
                representative[node.0] = *served;
            }
        }
        for node in &merged.nodes {
            mapping[node.0] = DraftIndex(twin);
        }
        for input in &merged.inputs {
            if let NodeInput::Node(child) = input {
                let DraftIndex(producer) = mapping[child.0];
                readers[producer].0 = readers[producer].0.saturating_sub(1);
            }
        }
        readers[twin].0 += readers[at].0;
    }
    merge
}
