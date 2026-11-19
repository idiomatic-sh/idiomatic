//! Spike (ryo-fmx3): the `partition` item of the `kernels` collection: one pass over a source that sorts the rows of the
//! constants its sibling filters ask for into one bucket per constant, in row order. A row whose key the filter's specialised
//! comparison cannot decide (no symbol, no text hash, no number) goes to the rest, grouped by its cell; each sibling decides
//! each rest group once with its own first comparison, on the group's first row, and reads the groups it does not fail with
//! its bucket. A row outside a sibling's bucket and outside the rest groups it reads fails the sibling's first comparison and
//! raises no fault, so reading the listed rows answers what scanning every row answers: same rows, order, duplicates, faults.
use crate::domain::comparing::service::number_of;
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::physical_plan::collections::kernels::filter::item::{filter, holds, readied};
use crate::domain::physical_plan::collections::kernels::scan_facts::item::{
    cell_of, count_of, facts, row_of, rows_of,
};
use crate::domain::physical_plan::collections::kernels::scan_roots::item::{roots, tree_read};
use crate::domain::physical_plan::collections::rewrites::partitioned_select::item::{
    PartitionGroup, PartitionKind,
};
use crate::domain::physical_plan::service::run::types::ChainRows;
use crate::domain::physical_plan::types::file_buffers::value::{FileBuffers, NodeDecisions};
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{InputRows, KernelRun, PlanNodeId, RowSource};
use crate::domain::patterns::types::PatternMatcher;
use crate::domain::table_texts::types::Texts;
use ryo_language::domain::symbols::service::hash_in;
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::rows::types::RowIndex;

/// How many distinct cells of the rest are told apart; the rest's other rows share the last group, which every sibling reads.
const REST_GROUPS: usize = 63;

/// One partition as the open file made it.
#[derive(Debug, Default)]
pub(crate) struct PartitionFile {
    /// The file (`generation` + 1) it was made in; 0 for none.
    stamp: u32,
    /// Which rows it was made of: where they are and how many.
    identity: (usize, usize),
    /// By slot, the rows whose key is the slot's constant, in row order.
    buckets: Vec<Vec<u32>>,
    /// The rows no key was read from, in row order, each with its rest group.
    rest: Vec<(u32, u8)>,
    /// Each rest group's cell and first row.
    groups: Vec<(Value, u32)>,
}

/// The partitions the open file made, by place, and the rows the last sibling to ask reads.
#[derive(Debug, Default)]
pub(crate) struct FilePartitions {
    /// By partition place.
    files: Vec<PartitionFile>,
    /// The rows a sibling reads, in row order: its bucket and the rest groups it reads, merged.
    pub(crate) rows: Vec<u32>,
    /// A row read from the rows, for a sibling's first comparison.
    scratch: Vec<Value>,
}

/// Which rows `rows` are: where they live and how many, so a partition made of one source is never read for another.
fn identity(rows: InputRows<'_>) -> (usize, usize) {
    match rows {
        InputRows::Facts(view) => (
            view.columns
                .first()
                .map_or(0, |column| column.0.as_ptr() as usize),
            rows_of(view).0,
        ),
        InputRows::Tree(tree) => (tree.candidates.as_ptr() as usize, tree.candidates.len()),
        InputRows::Table(table) => (table.cells.as_ptr() as usize, table.count.0),
        InputRows::Row(cells, _) => (cells.as_ptr() as usize, 1),
    }
}

/// The key the filter's specialised first comparison reads from `cell`; none when only its generic path decides it.
fn key_of(kind: PartitionKind, cell: Value, texts: &Texts<'_>) -> Option<u64> {
    match kind {
        PartitionKind::Text => {
            let Value::Symbol(symbol) = cell else {
                return None;
            };
            hash_in(texts.symbols, symbol).map(|hash| hash.0)
        }
        PartitionKind::Number => number_of(cell).map(i64::cast_unsigned),
    }
}

/// Makes partition `group` of `rows` for the open file into `file`.
fn made(group: &PartitionGroup, rows: InputRows<'_>, texts: &Texts<'_>, file: &mut PartitionFile, stamp: u32) {
    file.buckets.resize_with(group.keys.len(), Vec::new);
    for bucket in &mut file.buckets {
        bucket.clear();
    }
    file.rest.clear();
    file.groups.clear();
    file.identity = identity(rows);
    file.stamp = stamp;
    let Some(kind) = group.kind else {
        return;
    };
    let count = count_of(rows).0;
    for at in 0..count {
        let row = u32::try_from(at).unwrap_or(u32::MAX);
        let cell = cell_of(rows, RowIndex(at), group.column);
        match cell.and_then(|cell| key_of(kind, cell, texts)) {
            Some(key) => {
                if let Some(slot) = group.slots.get(&key)
                    && let Some(bucket) = file.buckets.get_mut(*slot as usize)
                {
                    bucket.push(row);
                }
            }
            None => {
                // A row past its cells, or past the told groups, is in the untold group every sibling reads.
                let place = match cell {
                    Some(cell) => match file.groups.iter().position(|(seen, _)| *seen == cell) {
                        Some(place) => place,
                        None if file.groups.len() < REST_GROUPS => {
                            file.groups.push((cell, row));
                            file.groups.len() - 1
                        }
                        None => REST_GROUPS,
                    },
                    None => REST_GROUPS,
                };
                file.rest.push((row, u8::try_from(place).unwrap_or(u8::MAX)));
            }
        }
    }
}

/// Whether partition `place` of the open file is made of `rows`, making it first when the file has not made it.
fn ensured(
    plan: &PhysicalPlan,
    place: u32,
    rows: InputRows<'_>,
    texts: &Texts<'_>,
    files: &mut Vec<PartitionFile>,
    stamp: u32,
) -> bool {
    let Some(group) = plan.partitions.groups.get(place as usize) else {
        return false;
    };
    let at = place as usize;
    if files.len() <= at {
        files.resize_with(at + 1, PartitionFile::default);
    }
    let Some(file) = files.get_mut(at) else {
        return false;
    };
    if file.stamp != stamp {
        made(group, rows, texts, file, stamp);
        return true;
    }
    file.identity == identity(rows)
}

/// The rest groups member `member` of partition `place` reads: those whose first row its first comparison does not fail (it
/// holds or faults); bit 63 (the rest's untold rows) always.
#[expect(clippy::too_many_arguments, reason = "spike (ryo-fmx3): the partition, the member, the rows and the decision state")]
fn rest_read(
    plan: &PhysicalPlan,
    place: u32,
    member: u32,
    file: &PartitionFile,
    rows: InputRows<'_>,
    texts: Texts<'_>,
    matcher: PatternMatcher<'_>,
    decisions: &mut NodeDecisions,
    scratch: &mut Vec<Value>,
) -> u64 {
    let always = 1u64 << REST_GROUPS;
    if file.groups.is_empty() {
        return always;
    }
    let Some(first) = plan
        .partitions
        .groups
        .get(place as usize)
        .and_then(|group| group.firsts.get(member as usize))
    else {
        return u64::MAX;
    };
    let kinds = crate::domain::physical_plan::collections::kernels::scan_facts::item::kinds_of(rows);
    let kernel = filter(first, kinds, texts);
    readied(decisions);
    let mut read = always;
    for (at, (_, row)) in file.groups.iter().enumerate() {
        let cells = row_of(rows, RowIndex(*row as usize), scratch);
        if !matches!(holds(kernel, matcher, cells, decisions), Ok(PredicateOutcome::Fails)) {
            read |= 1 << at;
        }
    }
    read
}

/// Fills `rows` with `file`'s bucket `slot` and the rest groups in `read`, merged in row order.
fn gathered(file: &PartitionFile, slot: u32, read: u64, rows: &mut Vec<u32>) {
    rows.clear();
    let bucket = file.buckets.get(slot as usize).map_or(&[][..], Vec::as_slice);
    let mut rest = file
        .rest
        .iter()
        .filter(|(_, group)| read & (1u64 << (*group).min(63)) != 0)
        .map(|(row, _)| *row)
        .peekable();
    let mut bucket = bucket.iter().copied().peekable();
    loop {
        let next = match (bucket.peek(), rest.peek()) {
            (Some(a), Some(b)) if a <= b => bucket.next(),
            (_, Some(_)) => rest.next(),
            (Some(_), None) => bucket.next(),
            (None, None) => break,
        };
        if let Some(row) = next {
            rows.push(row);
        }
    }
}

/// Whether `file`'s bucket `slot` holds no row and no rest row is in a group of `read`.
fn none_in(file: &PartitionFile, slot: u32, read: u64) -> bool {
    file.buckets.get(slot as usize).is_none_or(Vec::is_empty)
        && file.rest.iter().all(|(_, group)| read & (1u64 << (*group).min(63)) == 0)
}

/// The open file's stamp, as the run service stamps what it knows.
fn stamp_of(buffers: &FileBuffers) -> u32 {
    buffers.generation.wrapping_add(1)
}

/// What a sibling's partition answers over `rows`: made now when the file has not made it; `Some(true)` when the sibling
/// reads no row, `Some(false)` when it reads some (then listed in `partitions.rows` when `listed`), `None` when the partition
/// is not made of `rows`.
#[expect(clippy::too_many_arguments, reason = "spike (ryo-fmx3): the sibling, its rows and the buffers' parts it writes")]
fn answered(
    plan: &PhysicalPlan,
    (place, slot, member): (u32, u32, u32),
    rows: InputRows<'_>,
    texts: Texts<'_>,
    matcher: PatternMatcher<'_>,
    partitions: &mut FilePartitions,
    decisions: &mut NodeDecisions,
    stamp: u32,
    listed: bool,
) -> Option<bool> {
    let FilePartitions { files, rows: list, scratch } = partitions;
    if !ensured(plan, place, rows, &texts, files, stamp) {
        return None;
    }
    let file = files.get(place as usize)?;
    let read = rest_read(plan, place, member, file, rows, texts, matcher, decisions, scratch);
    if none_in(file, slot, read) {
        return Some(true);
    }
    if listed {
        gathered(file, slot, read, list);
    }
    Some(false)
}

/// Spike (ryo-fmx3): for a partitioned sibling `node`, made over `source`, its partition for the open file (made now when the
/// file has not made it) and, when `listed`, fills `buffers.partitions.rows` with the rows it reads. `Some(true)` when it reads
/// none (no row and, for a held node, no pending fault: it holds no row), `Some(false)` when it reads some, `None` when it is
/// no sibling or its source is not one partition's rows.
pub(crate) fn sibling_rows(
    plan: &PhysicalPlan,
    run: &KernelRun<'_>,
    node: PlanNodeId,
    source: RowSource,
    buffers: &mut FileBuffers,
    listed: bool,
) -> Option<bool> {
    let member = plan.partitions.of(node)?;
    let stamp = stamp_of(buffers);
    let texts = run.resources.texts();
    let FileBuffers {
        partitions,
        decisions,
        nodes,
        pending,
        ..
    } = buffers;
    let decisions = decisions.get_mut(usize::try_from(node.0).ok()?)?;
    match source {
        RowSource::Facts(relation) => {
            let view = facts(run.inputs, relation).ok()?;
            answered(plan, member, InputRows::Facts(&view), texts, run.matcher, partitions, decisions, stamp, listed)
        }
        RowSource::Tree(relation) => {
            let tree = roots(run.inputs, run.resources.tree(), node, relation);
            answered(plan, member, InputRows::Tree(&tree), texts, run.matcher, partitions, decisions, stamp, listed)
        }
        RowSource::Node(read) => {
            let at = usize::try_from(read.0).ok()?;
            if pending.get(at).is_some_and(Option::is_some) {
                return None;
            }
            let table = nodes.get(at)?.as_ref()?;
            answered(plan, member, InputRows::Table(table), texts, run.matcher, partitions, decisions, stamp, listed)
        }
        RowSource::Frontier(_) | RowSource::Observation(_) => None,
    }
}

/// Spike (ryo-fmx3): for a chain whose head stage `node` is a partitioned sibling, whether the open file's partition is made of
/// the head's rows; when it is, `buffers.partitions.rows` holds the rows the stage reads.
pub(crate) fn head_rows(
    plan: &PhysicalPlan,
    run: &KernelRun<'_>,
    node: PlanNodeId,
    head: &ChainRows<'_>,
    buffers: &mut FileBuffers,
) -> bool {
    let Some(member) = plan.partitions.of(node) else {
        return false;
    };
    let stamp = stamp_of(buffers);
    let made = buffers
        .partitions
        .files
        .get(member.0 as usize)
        .is_some_and(|file| file.stamp == stamp);
    if !made {
        return false;
    }
    let texts = run.resources.texts();
    let FileBuffers {
        partitions,
        decisions,
        ..
    } = buffers;
    let Some(decisions) = decisions.get_mut(usize::try_from(node.0).unwrap_or(usize::MAX)) else {
        return false;
    };
    let answer = match head {
        ChainRows::Facts(view) => {
            answered(plan, member, InputRows::Facts(view), texts, run.matcher, partitions, decisions, stamp, true)
        }
        ChainRows::Tree(roots) => {
            let tree = tree_read(*roots, run.resources.tree());
            answered(plan, member, InputRows::Tree(&tree), texts, run.matcher, partitions, decisions, stamp, true)
        }
        ChainRows::Node(_, taken) => answered(
            plan,
            member,
            InputRows::Table(&taken.rows),
            texts,
            run.matcher,
            partitions,
            decisions,
            stamp,
            true,
        ),
        ChainRows::Held(_) | ChainRows::Head | ChainRows::Right(_) | ChainRows::Untaken(_) => None,
    };
    match answer {
        Some(true) => {
            buffers.partitions.rows.clear();
            true
        }
        Some(false) => true,
        None => false,
    }
}
