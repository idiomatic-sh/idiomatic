//! The `table_scratch` service: giving back the room a big file made the operators grow, and asking of a set of kinds, with no IO and no ports.

use ryo_language::domain::relations::types::KindBits;

use super::types::KindSet;
use crate::domain::table_model::types::{KeyIndex, SetMembership, Table};
use crate::domain::table_scratch::types::OperatorScratch;

/// Actual allocated vector payload, excluding the inline Vec header in its owning structure.
pub(crate) fn vector_bytes<T>(values: &Vec<T>) -> usize {
    values.capacity().saturating_mul(size_of::<T>())
}

/// Conservative hash allocation bound: bucket payload, control bytes and alignment/group padding.
fn hashed_bytes<T>(capacity: usize) -> usize {
    if capacity == 0 {
        return 0;
    }
    capacity
        .saturating_add(1)
        .checked_next_power_of_two()
        .unwrap_or(usize::MAX)
        .saturating_mul(size_of::<T>().saturating_add(1))
        .saturating_add(64)
}

/// Capacity of an index, including grouping's hash-set allocation when present.
pub(crate) fn order_bytes(order: &KeyIndex) -> usize {
    [
        vector_bytes(&order.rows),
        vector_bytes(&order.hashes),
        vector_bytes(&order.heads),
        vector_bytes(&order.next),
        vector_bytes(&order.sorted),
        vector_bytes(&order.pairs),
        hashed_bytes::<crate::domain::table_model::types::KeyCellsHash>(order.runs.capacity()),
    ]
    .into_iter()
    .fold(0_usize, usize::saturating_add)
}

/// Exact table vector capacities; logical live length is accounted separately.
pub(crate) fn table_bytes(table: &Table) -> usize {
    vector_bytes(&table.cells).saturating_add(vector_bytes(&table.kinds))
}

/// Every reusable scratch allocation; vector bytes are exact, hash padding conservatively bounded.
pub(crate) fn scratch_bytes(scratch: &OperatorScratch) -> usize {
    use super::types::{
        Captured, ComparisonSlot, EntityNumber, KindSetPlace, KindTest, VerdictKey,
    };
    use crate::domain::globbing::types::CompiledGlobs;
    use crate::domain::table_cells::types::RowIndex;
    use crate::domain::table_model::types::Verdict;
    use ryo_language::domain::relations::types::Value;
    let vectors = [
        vector_bytes(&scratch.row),
        vector_bytes(&scratch.other),
        vector_bytes(&scratch.graph_walk.marks),
        vector_bytes(&scratch.graph_walk.reached),
        vector_bytes(&scratch.graph_walk.path),
        vector_bytes(&scratch.graph_walk.answers),
        scratch
            .graph_walk
            .queue
            .capacity()
            .saturating_mul(size_of::<u32>()),
        vector_bytes(&scratch.key),
        table_bytes(&scratch.right),
        table_bytes(&scratch.domain),
        order_bytes(&scratch.order),
        order_bytes(&scratch.domain_order),
        vector_bytes(&scratch.groups),
        vector_bytes(&scratch.walk.key),
        vector_bytes(&scratch.walk.frontier),
        vector_bytes(&scratch.walk.next),
        vector_bytes(&scratch.walk.reached),
        vector_bytes(&scratch.inner.key),
        vector_bytes(&scratch.inner.frontier),
        vector_bytes(&scratch.inner.next),
        vector_bytes(&scratch.inner.reached),
        vector_bytes(&scratch.dfs_graph.root_columns),
        vector_bytes(&scratch.dfs_graph.edge_columns),
        vector_bytes(&scratch.dfs_graph.adjacency),
        vector_bytes(&scratch.dfs_walk.key),
        vector_bytes(&scratch.dfs_walk.frames),
        vector_bytes(&scratch.places.0),
        scratch.text.0.capacity(),
        scratch.word.0.capacity(),
        vector_bytes(&scratch.hashes.0),
        vector_bytes(&scratch.needed.0),
        vector_bytes(&scratch.marks.0),
        vector_bytes(&scratch.kind_sets),
        vector_bytes(&scratch.kind_tests.0),
        vector_bytes(&scratch.gate.0),
        vector_bytes(&scratch.text_ok.0),
        vector_bytes(&scratch.candidates.0),
        vector_bytes(&scratch.text_starts.0),
        vector_bytes(&scratch.text_order.0),
        vector_bytes(&scratch.rest.0),
        vector_bytes(&scratch.nearest.0),
        vector_bytes(&scratch.climbing.0),
        vector_bytes(&scratch.captured),
        hashed_bytes::<RowIndex>(scratch.seen.capacity()),
        hashed_bytes::<Value>(scratch.walk.visited.capacity()),
        hashed_bytes::<Value>(scratch.inner.visited.capacity()),
        hashed_bytes::<Value>(scratch.dfs_walk.visited.capacity()),
        hashed_bytes::<(VerdictKey, Verdict)>(scratch.verdicts.capacity()),
        hashed_bytes::<(ComparisonSlot, CompiledGlobs)>(scratch.compiled_globs.capacity()),
        hashed_bytes::<(KindTest, KindSetPlace)>(scratch.kind_names.capacity()),
        hashed_bytes::<(Value, Captured)>(scratch.captures.capacity()),
        hashed_bytes::<(Value, EntityNumber)>(scratch.cycles.numbers.capacity()),
        vector_bytes(&scratch.cycles.values),
        vector_bytes(&scratch.cycles.pairs),
        vector_bytes(&scratch.cycles.starts.0),
        vector_bytes(&scratch.cycles.filled.0),
        vector_bytes(&scratch.cycles.leaving.0),
        vector_bytes(&scratch.cycles.order.0),
        vector_bytes(&scratch.cycles.low.0),
        vector_bytes(&scratch.cycles.open.0),
        vector_bytes(&scratch.cycles.stack.0),
        vector_bytes(&scratch.cycles.found.0),
        vector_bytes(&scratch.cycles.walk),
        vector_bytes(&scratch.cycles.component.0),
        vector_bytes(&scratch.cycles.ordered),
        hashed_bytes::<(
            crate::domain::table_texts::types::RegexPattern,
            regex_automata::meta::Regex,
        )>(scratch.compiled_regexes.0.capacity()),
    ];
    let mut bytes = vectors.into_iter().fold(0_usize, usize::saturating_add);
    for set in &scratch.kind_sets {
        bytes = bytes
            .saturating_add(vector_bytes(&set.bits.0))
            .saturating_add(vector_bytes(&set.kinds));
    }
    for glob in scratch.compiled_globs.values() {
        if let CompiledGlobs::Ready(regex) = glob {
            bytes = bytes.saturating_add(regex.memory_usage());
        }
    }
    for (pattern, regex) in &scratch.compiled_regexes.0 {
        bytes = bytes
            .saturating_add(pattern.0.capacity())
            .saturating_add(regex.memory_usage());
    }
    bytes
}

/// Clear file/rule population state only after all recursive operators/emitters have returned.
/// Compiled matchers and immutable kind sets may be retained within the aggregate allowance.
pub(crate) fn clear_scratch(scratch: &mut OperatorScratch) {
    // Marks retain only generation-neutral epochs, not rows/values; the next traversal advances the epoch.
    scratch.graph_walk.queue.clear();
    scratch.graph_walk.reached.clear();
    scratch.graph_walk.path.clear();
    scratch.graph_walk.answers.clear();
    scratch.row.clear();
    scratch.other.clear();
    scratch.key.clear();
    scratch.seen.clear();
    scratch.right.cells.clear();
    scratch.right.count.0 = 0;
    scratch.domain.cells.clear();
    scratch.domain.count.0 = 0;
    scratch.groups.clear();
    scratch.places.0.clear();
    scratch.text.0.clear();
    scratch.word.0.clear();
    scratch.hashes.0.clear();
    scratch.needed.0.clear();
    scratch.marks.0.clear();
    scratch.kind_tests.0.clear();
    scratch.gate.0.clear();
    scratch.text_ok.0.clear();
    scratch.candidates.0.clear();
    scratch.text_starts.0.clear();
    scratch.text_order.0.clear();
    scratch.text_file = Default::default();
    scratch.rest.0.clear();
    scratch.nearest.0.clear();
    scratch.climbing.0.clear();
    scratch.captures.clear();
    scratch.captured.clear();
    for walk in [&mut scratch.walk, &mut scratch.inner] {
        walk.key.clear();
        walk.frontier.clear();
        walk.next.clear();
        walk.reached.clear();
        walk.visited.clear();
    }
    for order in [&mut scratch.order, &mut scratch.domain_order] {
        order.rows.clear();
        order.hashes.clear();
        order.heads.clear();
        order.next.clear();
        order.sorted.clear();
        order.pairs.clear();
        order.runs.clear();
    }
    scratch.dfs_graph.root_columns.clear();
    scratch.dfs_graph.edge_columns.clear();
    scratch.dfs_graph.adjacency.clear();
    scratch.dfs_walk.key.clear();
    scratch.dfs_walk.frames.clear();
    scratch.dfs_walk.visited.clear();
    scratch.dfs_walk.pending = None;
    scratch.dfs_walk.ordinal = None;
    let cycle = &mut scratch.cycles;
    cycle.numbers.clear();
    cycle.values.clear();
    cycle.pairs.clear();
    cycle.starts.0.clear();
    cycle.filled.0.clear();
    cycle.leaving.0.clear();
    cycle.order.0.clear();
    cycle.low.0.clear();
    cycle.open.0.clear();
    cycle.stack.0.clear();
    cycle.found.0.clear();
    cycle.walk.clear();
    cycle.component.0.clear();
    cycle.ordered.clear();
}

/// Whether the kind numbered `kind` is in the set.
#[must_use]
pub(crate) fn kind_has(set: &KindSet, kind: KindBits) -> SetMembership {
    let kind = kind.0;
    let in_set = set
        .bits
        .0
        .get(usize::from(kind) / 64)
        .is_some_and(|word| (word >> (kind % 64)) & 1 == 1);
    if in_set {
        SetMembership::In
    } else {
        SetMembership::Out
    }
}
