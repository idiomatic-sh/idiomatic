//! Spike (ryo-vhuy): the typed chain path. See `typed.rs`.
use super::super::super::super::super::types::lowering::value::Lowering;
use super::super::super::types::{ChainRows, Push};
use super::super::sources::service::{physical, resolved_rows};
use crate::domain::comparing::service::{FastComparison, fast_holds, fast_of};
use crate::domain::contract::types::{PortableOperatorFault, PredicateOutcome};
use crate::domain::equality_join::types::{ProbeKey, ProbeMode};
use crate::domain::physical_plan::collections::kernels::distinct::item::counted as repeats_counted;
use crate::domain::physical_plan::collections::kernels::scan_facts::item::facts_cell_at;
use crate::domain::physical_plan::collections::kernels::scan_node::item::fed_with;
use crate::domain::physical_plan::service::run::counting::service::{counted, timed, timed_to};
use crate::domain::physical_plan::types::node_counters::value::MadeIn;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalNode;
use crate::domain::physical_plan::types::stream_scratch::value::StageScratch;
use crate::domain::physical_plan::types::{CellStart, NodeFed, PlanNodeId, ProbeIndex, RowSource};
use ryo_relations::domain::relations::types::{CellKind, ColumnsView, Value};
use ryo_values::domain::plan_parts::types::{RepeatedRows, Side};
use ryo_values::domain::rows::types::{RowCount, RowIndex, Table};
use std::cell::RefCell;

/// Why a node or a chain is not typed; `ELIGIBLE` (0) for a stage the typed path covers.
pub(crate) const REASONS: [&str; 16] = [
    "eligible",
    "no_layout",
    "tree_or_host_source",
    "probe_shape",
    "extend",
    "text_map",
    "generic_comparison",
    "bindings",
    "kind_undecided_at_plan",
    "other_kernel",
    "head_tree",
    "prepare_fault",
    "cell_undecided",
    "probe_not_head",
    "dropped_not_tail",
    "probe_off",
];
/// A stage the typed path covers.
pub(crate) const ELIGIBLE: u8 = 0;
/// A column of a probe's right rows, in a stage's column map; any other entry is a column of the head's rows.
const RIGHT: usize = 1 << 20;
/// No right row: a semi or anti probe's answer, or a chain with no probe.
const NONE: u32 = u32::MAX;
/// Positions past this are not addressed by a position array: the chain runs today's path.
const POSITIONS: usize = 1 << 22;
/// Symbol ids past this are decided without the per-run memo.
const MEMO: usize = 1 << 18;

/// The input column kinds of a node reading `source`, from the plan's layouts or the relation's catalog.
fn source_kinds<'k>(
    source: RowSource,
    layouts: &'k [Option<Vec<CellKind>>],
) -> Result<&'k [CellKind], u8> {
    match source {
        RowSource::Facts(relation) => {
            Ok(ryo_relations::domain::relations::service::kinds(relation))
        }
        RowSource::Node(node) => layouts
            .get(usize::try_from(node.0).unwrap_or(usize::MAX))
            .and_then(Option::as_deref)
            .ok_or(1),
        RowSource::Tree(_) | RowSource::Frontier(_) | RowSource::Observation(_) => Err(2),
    }
}

/// Whether `fast`, over input columns of `kinds`, is decided by its specialisation for every cell of those kinds.
fn decided(fast: FastComparison<'_>, kinds: &[CellKind]) -> bool {
    let kind = |at: usize| kinds.get(at).copied();
    match fast {
        FastComparison::SymbolText { column, .. } => kind(column) == Some(CellKind::Symbol),
        FastComparison::NumberLiteral { column, .. } => {
            matches!(kind(column), Some(CellKind::Number | CellKind::Node))
        }
        FastComparison::Columns { left, right, .. } => matches!(
            (kind(left), kind(right)),
            (Some(CellKind::Node), Some(CellKind::Node))
                | (Some(CellKind::Number), Some(CellKind::Number))
        ),
        FastComparison::Generic => false,
    }
}

/// By node, at plan time: `ELIGIBLE` for a filter, projection or id-keyed probe the typed path covers, else why not.
pub(crate) fn eligibility(nodes: &[PhysicalNode], layouts: &[Option<Vec<CellKind>>]) -> Vec<u8> {
    nodes
        .iter()
        .map(|node| match &node.lowering {
            Lowering::Filter { source, select } => {
                let kinds = match source_kinds(*source, layouts) {
                    Ok(kinds) => kinds,
                    Err(reason) => return reason,
                };
                if !select.bindings.is_empty() {
                    return 7;
                }
                for comparison in &select.comparisons {
                    let fast = fast_of(comparison);
                    if matches!(fast, FastComparison::Generic) {
                        return 6;
                    }
                    if !decided(fast, kinds) {
                        return 8;
                    }
                }
                ELIGIBLE
            }
            Lowering::Project { source, .. } => match source_kinds(*source, layouts) {
                Ok(_) => ELIGIBLE,
                Err(reason) => reason,
            },
            Lowering::Probe {
                left,
                right,
                probe,
                index,
            } => {
                let (Ok(left_kinds), Ok(right_kinds)) =
                    (source_kinds(*left, layouts), source_kinds(*right, layouts))
                else {
                    return 2;
                };
                let one_id = match &probe.key {
                    ProbeKey::Columns { left: l, right: r } if l.len() == 1 && r.len() == 1 => {
                        let at = |c: &ryo_values::domain::plan_parts::types::ColumnIndex| {
                            usize::try_from(c.0).unwrap_or(usize::MAX)
                        };
                        left_kinds.get(at(&l[0])) == Some(&CellKind::Node)
                            && right_kinds.get(at(&r[0])) == Some(&CellKind::Node)
                    }
                    _ => false,
                };
                let shaped = one_id
                    && matches!(
                        probe.mode,
                        ProbeMode::Inner | ProbeMode::Semi | ProbeMode::Anti
                    )
                    && probe.filter.comparisons.is_empty()
                    && probe.filter.bindings.is_empty()
                    && probe.filter.kept.is_empty()
                    && *index != ProbeIndex::Tree;
                if shaped { ELIGIBLE } else { 3 }
            }
            Lowering::Extend { .. } => 4,
            Lowering::TextMap { .. } => 5,
            _ => 9,
        })
        .collect()
}

/// A position-addressed index of a probe's right rows by their node key: for each key position, its rows ascending, valid
/// where `stamp` holds this run's generation; `ids` the whole key of each right row, compared on a match. The worker's,
/// never cleared.
#[derive(Default)]
struct Positions {
    generation: u32,
    stamp: Vec<u32>,
    head: Vec<u32>,
    next: Vec<u32>,
    ids: Vec<u64>,
}

/// The worker's typed scratch.
#[derive(Default)]
struct Scratch {
    /// The selection: each kept row's head row and right row (`NONE` for none).
    left: Vec<u32>,
    right: Vec<u32>,
    generation: u32,
    /// By comparison slot: the generation each symbol was decided in, and its verdict.
    memo: Vec<(Vec<u32>, Vec<bool>)>,
    index: Positions,
    /// Spike (ryo-vhuy, `repeats`): the tail rows kept so far, by their cells' words and each cell's variant.
    seen: rustc_hash::FxHashSet<([u64; 4], u32)>,
}

thread_local! {
    static SCRATCH: RefCell<Scratch> = RefCell::new(Scratch::default());
}

/// One comparison of the chain, on the head's and the right rows' columns.
struct Compiled<'c> {
    fast: FastComparison<'c>,
    /// The stage it belongs to.
    stage: usize,
}

/// Rows read in place.
#[derive(Clone, Copy)]
enum Rows<'h> {
    Facts(&'h ColumnsView<'h>),
    Table(&'h Table),
}
impl<'h> Rows<'h> {
    fn of(rows: &'h ChainRows<'_>) -> Option<Self> {
        match rows {
            ChainRows::Facts(view) => Some(Self::Facts(view)),
            ChainRows::Node(_, taken) => Some(Self::Table(&taken.rows)),
            ChainRows::Held(table) => Some(Self::Table(table)),
            ChainRows::Tree(_) | ChainRows::Head | ChainRows::Right(_) | ChainRows::Untaken(_) => {
                None
            }
        }
    }
    fn count(self) -> usize {
        match self {
            Self::Facts(view) => view.columns.first().map_or(0, |column| column.0.len()),
            Self::Table(table) => table.count.0,
        }
    }
    fn width(self) -> usize {
        match self {
            Self::Facts(view) => view.kinds.len(),
            Self::Table(table) => table.kinds.len(),
        }
    }
    #[inline]
    fn cell(self, row: usize, column: usize) -> Option<Value> {
        match self {
            Self::Facts(view) => facts_cell_at(view, RowIndex(row), column),
            Self::Table(table) => {
                let width = table.kinds.len();
                if column < width && row < table.count.0 {
                    table.cells.get(row * width + column).copied()
                } else {
                    None
                }
            }
        }
    }
}

/// Spike (ryo-vhuy, `repeats`): a cell as one word and its variant; none for a cell that is no word (a name from another file).
#[inline]
fn word(value: Value) -> Option<(u64, u32)> {
    match value {
        Value::Node(id) => Some((id.0, 1)),
        Value::Symbol(symbol) => Some((u64::from(symbol.0), 2)),
        Value::File(file) => Some((u64::from(file.0), 3)),
        Value::Span(span) => Some(((u64::from(span.start.0) << 32) | u64::from(span.end.0), 4)),
        Value::Number(number) => Some((number.0.cast_unsigned(), 5)),
        Value::Flag(flag) => Some((
            u64::from(flag == ryo_relations::domain::relations::types::Flag::Lowered),
            6,
        )),
        Value::Name(_) => None,
    }
}

/// The cell of a column-map entry for a selected pair.
#[inline]
fn pair_cell(
    head: Rows<'_>,
    right: Option<Rows<'_>>,
    left_row: usize,
    right_row: u32,
    entry: usize,
) -> Option<Value> {
    if entry & RIGHT == 0 {
        head.cell(left_row, entry)
    } else {
        right?.cell(right_row as usize, entry & !RIGHT)
    }
}

fn count_reason(reason: u8, rows: usize) {
    crate::spike::typed_count(usize::from(reason), rows as u64);
}

/// The probe at the chain's head, as the typed loop reads it.
struct HeadProbe<'p> {
    mode: ProbeMode,
    left_key: usize,
    right_key: usize,
    right: Rows<'p>,
}

/// Pushes the chain's head rows through its stages into the tail as one typed loop. `None` when the typed path does not
/// cover this chain (the reason counted): the tail and every stage are untouched and today's path runs.
pub(in crate::domain::physical_plan::service::run::pipeline) fn pushed(
    push: &mut Push<'_, '_, '_>,
    nodes: &[PlanNodeId],
    stages: &mut [StageScratch],
) -> Option<Result<Option<PortableOperatorFault>, PortableOperatorFault>> {
    if !crate::spike::typed_lever("chains") {
        return None;
    }
    let plan = push.plan;
    for node in nodes {
        let reason = plan.typed_stage(*node);
        if reason != ELIGIBLE {
            count_reason(reason, 0);
            return None;
        }
    }
    let Some(head) = Rows::of(push.head) else {
        count_reason(10, 0);
        return None;
    };
    if push.hard.is_some() || stages.iter().any(|stage| stage.fault.is_some()) {
        count_reason(11, 0);
        return None;
    }
    let width = head.width();
    let last = nodes.len().checked_sub(1)?;
    // Each stage's columns as the head's (or the head probe's right rows') columns, and the comparisons on them.
    let mut map: Vec<usize> = (0..width).collect();
    let mut compiled: Vec<Compiled<'_>> = Vec::new();
    let mut tail_distinct = false;
    let mut probe: Option<HeadProbe<'_>> = None;
    for (at, node) in nodes.iter().enumerate() {
        let mapped = |map: &[usize], column: u32| {
            map.get(usize::try_from(column).unwrap_or(usize::MAX))
                .copied()
        };
        match &physical(plan, *node).ok()?.lowering {
            Lowering::Filter { select, .. } => {
                for comparison in &select.comparisons {
                    let m = |column: usize| map.get(column).copied();
                    let fast = match fast_of(comparison) {
                        FastComparison::SymbolText {
                            column,
                            equal,
                            hash,
                            text,
                        } => FastComparison::SymbolText {
                            column: m(column)?,
                            equal,
                            hash,
                            text,
                        },
                        FastComparison::NumberLiteral {
                            column,
                            test,
                            literal,
                        } => FastComparison::NumberLiteral {
                            column: m(column)?,
                            test,
                            literal,
                        },
                        FastComparison::Columns { left, right, test } => FastComparison::Columns {
                            left: m(left)?,
                            right: m(right)?,
                            test,
                        },
                        FastComparison::Generic => return None,
                    };
                    compiled.push(Compiled { fast, stage: at });
                }
                if !select.kept.is_empty() {
                    map = select
                        .kept
                        .iter()
                        .map(|column| mapped(&map, column.0))
                        .collect::<Option<_>>()?;
                }
            }
            Lowering::Project { project, .. } => {
                map = project
                    .columns
                    .iter()
                    .map(|column| mapped(&map, column.0))
                    .collect::<Option<_>>()?;
                if project.distinct == RepeatedRows::Dropped {
                    if at != last {
                        count_reason(14, 0);
                        return None;
                    }
                    tail_distinct = true;
                }
            }
            Lowering::Probe { probe: parts, .. } => {
                if at != 0 {
                    count_reason(13, 0);
                    return None;
                }
                if !crate::spike::typed_lever("joins") {
                    count_reason(15, 0);
                    return None;
                }
                let ProbeKey::Columns { left, right } = &parts.key else {
                    return None;
                };
                let right_rows = push.rights.get(stages.first()?.own.right.0)?;
                let Some(right_rows) = Rows::of(resolved_rows(right_rows, push.head, push.rights))
                else {
                    count_reason(10, 0);
                    return None;
                };
                if parts.repeated == RepeatedRows::Dropped {
                    if at != last {
                        count_reason(14, 0);
                        return None;
                    }
                    tail_distinct = true;
                }
                if parts.mode == ProbeMode::Inner {
                    map = parts
                        .output
                        .iter()
                        .map(|column| match column.side {
                            Side::Left => mapped(&map, column.column.0),
                            Side::Right => usize::try_from(column.column.0)
                                .ok()
                                .filter(|c| *c < RIGHT)
                                .map(|c| c | RIGHT),
                        })
                        .collect::<Option<_>>()?;
                }
                probe = Some(HeadProbe {
                    mode: parts.mode,
                    left_key: usize::try_from(left.first()?.0).ok()?,
                    right_key: usize::try_from(right.first()?.0).ok()?,
                    right: right_rows,
                });
            }
            _ => return None,
        }
    }
    let count = head.count();
    let right = probe.as_ref().map(|probe| probe.right);
    let first = usize::from(probe.is_some());
    let texts = push.run.resources.texts();
    let span = timed(push.buffers);
    let mut reads = vec![0_usize; nodes.len()];
    let decided = SCRATCH.with(|scratch| {
        let scratch = &mut *scratch.borrow_mut();
        scratch.left.clear();
        scratch.right.clear();
        if scratch.memo.len() < compiled.len() {
            scratch.memo.resize_with(compiled.len(), Default::default);
        }
        scratch.generation = scratch.generation.wrapping_add(1);
        if scratch.generation == 0 {
            for (stamps, _) in &mut scratch.memo {
                stamps.fill(0);
            }
            scratch.generation = 1;
        }
        let generation = scratch.generation;
        // The head probe's right rows, indexed by their key's position.
        if let Some(probe) = &probe {
            let index = &mut scratch.index;
            index.generation = index.generation.wrapping_add(1);
            if index.generation == 0 {
                index.stamp.fill(0);
                index.generation = 1;
            }
            let rows = probe.right.count();
            if index.next.len() < rows {
                index.next.resize(rows, NONE);
                index.ids.resize(rows, 0);
            }
            for row in (0..rows).rev() {
                let Value::Node(id) = probe.right.cell(row, probe.right_key)? else {
                    return None;
                };
                let at = (id.0 & 0xffff_ffff) as usize;
                if at >= POSITIONS {
                    return None;
                }
                if at >= index.stamp.len() {
                    index.stamp.resize(at + 1, 0);
                    index.head.resize(at + 1, NONE);
                }
                if index.stamp[at] != index.generation {
                    index.stamp[at] = index.generation;
                    index.head[at] = NONE;
                }
                index.next[row] = index.head[at];
                index.ids[row] = id.0;
                index.head[at] = u32::try_from(row).ok()?;
            }
        }
        let mut candidates: Vec<u32> = Vec::new();
        // Pass one: the selection of (head row, right row) pairs every stage keeps.
        for row in 0..count {
            candidates.clear();
            match &probe {
                None => candidates.push(NONE),
                Some(probe) => {
                    reads[0] += 1;
                    let Value::Node(id) = head.cell(row, probe.left_key)? else {
                        return None;
                    };
                    let at = (id.0 & 0xffff_ffff) as usize;
                    let index = &scratch.index;
                    let mut found = if at < index.stamp.len() && index.stamp[at] == index.generation
                    {
                        index.head[at]
                    } else {
                        NONE
                    };
                    let mut any = false;
                    while found != NONE {
                        if index.ids[found as usize] == id.0 {
                            any = true;
                            if probe.mode != ProbeMode::Inner {
                                break;
                            }
                            candidates.push(found);
                        }
                        found = index.next[found as usize];
                    }
                    match probe.mode {
                        ProbeMode::Semi if any => candidates.push(NONE),
                        ProbeMode::Anti if !any => candidates.push(NONE),
                        _ => {}
                    }
                }
            }
            'pairs: for pair in &candidates {
                let mut stage = first;
                if stage <= last {
                    reads[stage] += 1;
                }
                for (slot, comparison) in compiled.iter().enumerate() {
                    while stage < comparison.stage {
                        stage += 1;
                        reads[stage] += 1;
                    }
                    let outcome = match comparison.fast {
                        FastComparison::SymbolText { column, .. } => {
                            let value = pair_cell(head, right, row, *pair, column)?;
                            let Value::Symbol(symbol) = value else {
                                return None;
                            };
                            let at = usize::try_from(symbol.0).unwrap_or(usize::MAX);
                            if at >= MEMO {
                                // A symbol id past the memo's room (a vocabulary or derived id): decided on its own.
                                fast_holds(&comparison.fast, |_| Some(value), &texts)?
                                    == PredicateOutcome::Holds
                            } else {
                                let (stamps, verdicts) = &mut scratch.memo[slot];
                                if at >= stamps.len() {
                                    stamps.resize(at + 1, 0);
                                    verdicts.resize(at + 1, false);
                                }
                                if stamps[at] == generation {
                                    verdicts[at]
                                } else {
                                    let holds =
                                        fast_holds(&comparison.fast, |_| Some(value), &texts)?
                                            == PredicateOutcome::Holds;
                                    stamps[at] = generation;
                                    verdicts[at] = holds;
                                    holds
                                }
                            }
                        }
                        _ => {
                            fast_holds(
                                &comparison.fast,
                                |column| pair_cell(head, right, row, *pair, column),
                                &texts,
                            )? == PredicateOutcome::Holds
                        }
                    };
                    if !outcome {
                        continue 'pairs;
                    }
                }
                while stage < last {
                    stage += 1;
                    reads[stage] += 1;
                }
                scratch.left.push(u32::try_from(row).ok()?);
                scratch.right.push(*pair);
            }
        }
        // Pass two: the tail's cells of each selected pair, repeats dropped as the tail's own repeat set drops them. Spike
        // (ryo-vhuy, `repeats`): a tail of at most four columns whose every selected cell is one word drops its repeats by
        // those words in the worker's set instead, keeping the same first rows.
        let output = &mut *push.tail;
        let words = tail_distinct
            && map.len() <= 4
            && crate::spike::typed_lever("repeats")
            && scratch
                .left
                .iter()
                .zip(&scratch.right)
                .all(|(left_row, right_row)| {
                    map.iter().all(|entry| {
                        pair_cell(head, right, *left_row as usize, *right_row, *entry)
                            .and_then(word)
                            .is_some()
                    })
                });
        if words {
            scratch.seen.clear();
            for (left_row, right_row) in scratch.left.iter().zip(&scratch.right) {
                let mut key = ([0_u64; 4], 0_u32);
                for (slot, entry) in map.iter().enumerate() {
                    let (bits, variant) =
                        pair_cell(head, right, *left_row as usize, *right_row, *entry)
                            .and_then(word)?;
                    key.0[slot] = bits;
                    key.1 |= variant << (3 * slot);
                }
                if scratch.seen.insert(key) {
                    for entry in &map {
                        output.cells.push(pair_cell(
                            head,
                            right,
                            *left_row as usize,
                            *right_row,
                            *entry,
                        )?);
                    }
                    output.count = RowCount(output.count.0 + 1);
                }
            }
            return Some(scratch.left.len());
        }
        let repeats = &mut stages.get_mut(last)?.own.repeats;
        for (left_row, right_row) in scratch.left.iter().zip(&scratch.right) {
            let start = CellStart(output.cells.len());
            for entry in &map {
                output.cells.push(pair_cell(
                    head,
                    right,
                    *left_row as usize,
                    *right_row,
                    *entry,
                )?);
            }
            if tail_distinct {
                repeats_counted(output, start, RepeatedRows::Dropped, repeats);
            } else {
                output.count = RowCount(output.count.0 + 1);
            }
        }
        Some(scratch.left.len())
    });
    timed_to(push.buffers, span, nodes[0]);
    let Some(kept) = decided else {
        // Pass one stopped before any cell was written (pass two never fails on a pair pass one kept): nothing to undo.
        count_reason(12, 0);
        return None;
    };
    if kept > 0 {
        crate::spike::typed_count(REASONS.len(), kept as u64);
    }
    for (at, (node, stage)) in nodes.iter().zip(stages.iter_mut()).enumerate() {
        stage.read = RowCount(reads[at]);
        let right_fed = if at == 0 && probe.is_some() {
            stage.own.fed
        } else {
            RowCount(0)
        };
        fed_with(
            push.buffers,
            *node,
            NodeFed([RowCount(reads[at]), right_fed, RowCount(0)]),
        );
        if let Some(next) = reads.get(at + 1) {
            counted(push.buffers, *node, MadeIn::Chain, RowCount(*next));
        }
    }
    count_reason(ELIGIBLE, count);
    Some(Ok(None))
}
