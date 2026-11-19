//! Prepare shared equality indexes and evaluate their rows in source order.
use super::types::SelectionInput;
use super::types::{
    Access, Counts, Equality, File, Instruction, LiteralId, MatchDemand, Partition, Positions,
    PredicatePartitionId, PredicateSelection, Program, Subscription, TestIndex,
};
use crate::domain::contract::types::PredicateOutcome;
use rustc_hash::FxHashMap;
use ryo_relations::domain::relations::types::Bits;
use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};
use ryo_values::domain::rows::types::RowIndex;
use ryo_values::domain::table_words::types::{ColumnCount, WordSlice};
use std::collections::BTreeMap;

/// Plan at most one ordered partition for each selection. Prefer the largest sibling group.
pub fn prepare(selections: Vec<PredicateSelection>) -> Program {
    let mut groups = BTreeMap::<(StepId, ColumnIndex), BTreeMap<StepId, LiteralId>>::new();
    for (ordinal, selection) in selections.iter().enumerate() {
        let SelectionInput::Derived(source) = selection.source else {
            continue;
        };
        let Ok(node) = u32::try_from(ordinal).map(StepId) else {
            continue;
        };
        for test in &selection.instructions {
            if let Instruction::Word(column, literal, Equality::Equal) = test {
                groups
                    .entry((source, *column))
                    .or_default()
                    .entry(node)
                    .or_insert(*literal);
            }
        }
    }
    let mut groups: Vec<_> = groups
        .into_iter()
        .filter(|(_, readers)| readers.len() > 1)
        .collect();
    groups.sort_by_key(|(key, readers)| (std::cmp::Reverse(readers.len()), *key));
    let mut subscriptions = vec![None; selections.len()];
    let mut releases = vec![Vec::new(); selections.len()];
    let mut partitions = Vec::new();
    for ((source, column), readers) in groups {
        let candidates: Vec<_> = readers
            .into_iter()
            .filter(|(node, _)| subscriptions[usize::from(*node)].is_none())
            .collect();
        if candidates.len() < 2 {
            continue;
        }
        let partition = PredicatePartitionId(partitions.len());
        let mut literals = Vec::with_capacity(candidates.len());
        for (node, literal) in candidates {
            subscriptions[usize::from(node)] = Some(Subscription { partition, literal });
            literals.push(literal);
        }
        literals.sort_unstable();
        literals.dedup();
        partitions.push(Partition { column, literals });
        releases[usize::from(source)].push(partition);
    }
    Program {
        selections,
        partitions,
        subscriptions,
        releases,
    }
}

/// Bind the program to the caller's complete canonical namespace for one file.
pub fn bind(plan: &Program, words: Vec<Option<Bits>>, access: Access) -> File<'_> {
    File {
        plan,
        words,
        access,
        counts: Counts::default(),
        positions: (0..plan.partitions.len())
            .map(|_| Positions::Unread)
            .collect(),
    }
}

/// Drop a shared index when its immutable parent ceases to exist.
pub fn release(file: &mut File<'_>, source: StepId) {
    for partition in &file.plan.releases[usize::from(source)] {
        file.positions[partition.0] = Positions::Unread;
    }
}

/// Evaluate the conjunction in authored order, including each delegated test at its original position.
pub fn matches_row(
    file: &File<'_>,
    node: StepId,
    row: WordSlice<'_>,
    mut original: impl FnMut(TestIndex) -> PredicateOutcome,
) -> PredicateOutcome {
    for instruction in &file.plan.selections[usize::from(node)].instructions {
        let keep = match *instruction {
            Instruction::Word(column, literal, required) => {
                (Some(Bits(row.0[usize::from(column)])) == file.words[literal.0])
                    == (required == Equality::Equal)
            }
            Instruction::Flag(column, when_false, when_true) => {
                (if row.0[usize::from(column)] == 0 {
                    when_false
                } else {
                    when_true
                }) == PredicateOutcome::Holds
            }
            Instruction::Same(left, right, required) => {
                (row.0[usize::from(left)] == row.0[usize::from(right)])
                    == (required == Equality::Equal)
            }
            Instruction::Original(test) => original(test) == PredicateOutcome::Holds,
        };
        if !keep {
            return PredicateOutcome::Fails;
        }
    }
    PredicateOutcome::Holds
}

/// Read the demanded rows from an immutable input with a positive, divisive width.
/// The caller lends its comparator and consumes each answer before the next visit.
pub fn select(
    file: &mut File<'_>,
    node: StepId,
    input: WordSlice<'_>,
    width: ColumnCount,
    demand: MatchDemand,
    mut original: impl FnMut(TestIndex, WordSlice<'_>) -> PredicateOutcome,
    mut emit: impl FnMut(WordSlice<'_>),
) {
    let count = input.0.len() / width.0;
    file.counts.selections.0 += 1;
    file.counts.input_positions.0 += count;
    let mut lookup = None;
    if file.access == Access::Shared
        && let Some(reader) = file.plan.subscriptions[usize::from(node)]
    {
        let Some(word) = file.words[reader.literal.0] else {
            file.counts.absent.0 += 1;
            return;
        };
        let slot = &mut file.positions[reader.partition.0];
        match slot {
            Positions::Unread => *slot = Positions::ReadOnce,
            Positions::ReadOnce => {
                let partition = &file.plan.partitions[reader.partition.0];
                let mut entries: FxHashMap<_, Vec<_>> = partition
                    .literals
                    .iter()
                    .filter_map(|key| file.words[key.0].map(|word| (word, Vec::new())))
                    .collect();
                for (index, row) in input.0.chunks_exact(width.0).enumerate() {
                    if let Some(positions) =
                        entries.get_mut(&Bits(row[usize::from(partition.column)]))
                    {
                        positions.push(RowIndex(index));
                    }
                }
                *slot = Positions::Indexed(entries);
                file.counts.builds.0 += 1;
                file.counts.build_positions.0 += count;
            }
            Positions::Indexed(_) => {}
        }
        if matches!(slot, Positions::Indexed(_)) {
            lookup = Some((reader.partition, word))
        }
    }
    let mut visited = 0;
    let mut deliver = |row| {
        visited += 1;
        if matches_row(file, node, row, |test| original(test, row)) == PredicateOutcome::Holds {
            emit(row);
            return demand == MatchDemand::First;
        }
        false
    };
    if let Some((partition, word)) = lookup
        && let Positions::Indexed(entries) = &file.positions[partition.0]
    {
        if let Some(positions) = entries.get(&word) {
            for index in positions {
                let start = index.0 * width.0;
                if deliver(WordSlice(&input.0[start..start + width.0])) {
                    break;
                }
            }
        }
        file.counts.indexed_reads.0 += 1;
    } else {
        for row in input.0.chunks_exact(width.0) {
            if deliver(WordSlice(row)) {
                break;
            }
        }
    }
    file.counts.inspected.0 += visited;
}
