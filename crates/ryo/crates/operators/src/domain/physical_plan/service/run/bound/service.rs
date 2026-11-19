//! Spike (ryo-22c7): restricted evaluation. At an inner, semi or anti probe whose left rows are made and whose right side is a
//! node not yet made in the open file, the right side is made only for the key values the left rows hold.
use crate::domain::contract::types::PortableOperatorFault;
use crate::domain::equality_join::types::ProbeParts;
use crate::domain::physical_plan::collections::kernels::scan_facts::item::{
    count_of, facts, row_of,
};
use crate::domain::physical_plan::collections::kernels::scan_roots::item::{rooted, tree_read};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::{InputRows, KernelRun, NodeRows, PlanNodeId, RowSource};
use ryo_relations::domain::relations::types::Value;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::rows::types::RowIndex;

/// The distinct key values of `source`'s rows on `on` (every column when `on` is none), at most `cap` of them; none past the
/// cap or for a source that cannot be read here. Each key is its cells, in first-occurrence order.
pub(crate) fn left_keys(
    run: &KernelRun<'_>,
    reader: PlanNodeId,
    source: RowSource,
    on: Option<&[ColumnIndex]>,
    buffers: &mut FileBuffers,
    cap: usize,
    keys: &mut Vec<Vec<Value>>,
) -> Result<bool, PortableOperatorFault> {
    keys.clear();
    let mut scratch = std::mem::take(&mut buffers.row);
    let answer = match source {
        RowSource::Node(read) => match buffers.rows(read) {
            NodeRows::Materialized(table) => Ok(collect(InputRows::Table(table), on, cap, keys, &mut scratch)),
            NodeRows::Absent => Ok(false),
        },
        RowSource::Facts(relation) => {
            let view = facts(run.inputs, relation)?;
            Ok(collect(InputRows::Facts(&view), on, cap, keys, &mut scratch))
        }
        RowSource::Tree(relation) => {
            let roots = rooted(run.inputs, reader, relation);
            let tree = tree_read(roots, run.resources.tree());
            Ok(collect(InputRows::Tree(&tree), on, cap, keys, &mut scratch))
        }
        RowSource::Frontier(_) | RowSource::Observation(_) => Ok(false),
    };
    buffers.row = scratch;
    answer
}

fn collect(
    rows: InputRows<'_>,
    on: Option<&[ColumnIndex]>,
    cap: usize,
    keys: &mut Vec<Vec<Value>>,
    scratch: &mut Vec<Value>,
) -> bool {
    let mut seen: HashSet<Vec<Value>> = HashSet::new();
    for row in 0..count_of(rows).0 {
        let cells = row_of(rows, RowIndex(row), scratch);
        let key: Vec<Value> = match on {
            Some(on) => {
                let mut key = Vec::with_capacity(on.len());
                for column in on {
                    let Some(cell) = cells.get(usize::try_from(column.0).unwrap_or(usize::MAX)) else {
                        return false;
                    };
                    key.push(*cell);
                }
                key
            }
            None => cells.to_vec(),
        };
        if seen.insert(key.clone()) {
            if keys.len() >= cap {
                return false;
            }
            keys.push(key);
        }
    }
    true
}

/// The left key columns of a probe; none for a whole-row key.
pub(crate) fn left_on(probe: &ProbeParts) -> Option<&[ColumnIndex]> {
    match &probe.key {
        ProbeKey::Columns { left, .. } => Some(left),
        ProbeKey::Whole => None,
    }
}

// ---- restricted evaluation (Spike ryo-22c7) ----

use crate::domain::physical_plan::collections::kernels::scan_facts::item::{cell_of, kinds_of};
use crate::domain::equality_join::types::ProbeKey;
use crate::domain::physical_plan::service::kernels::service::kernels;
use crate::domain::physical_plan::service::run::pipeline::stages::service::{stage_chunk, stage_prepared};
use crate::domain::physical_plan::service::run::service::{Known, guarded_empty, known, marked_empty};
use crate::domain::physical_plan::service::run::types::{ChainRows, ChainStage, KernelInputs};
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::stream_scratch::value::{CellBound, ChunkStop, RowChunk, StageOwn};
use crate::domain::physical_plan::types::ProbeIndex;
use crate::domain::equality_join::types::ProbeMode;
use crate::domain::kept_room::constants::POOL_TABLES;
use crate::domain::kept_room::types::PoolSize;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::rows::types::{RowCount, Table};
use std::cell::RefCell;
use std::collections::HashSet;

/// Spike (ryo-22c7): a multiply-rotate hasher for key cells (the run hashes its own numbers, never untrusted input).
#[derive(Default)]
pub(crate) struct Quick(u64);

impl std::hash::Hasher for Quick {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for byte in bytes {
            self.write_u64(u64::from(*byte));
        }
    }
    fn write_u8(&mut self, value: u8) {
        self.write_u64(u64::from(value));
    }
    fn write_u16(&mut self, value: u16) {
        self.write_u64(u64::from(value));
    }
    fn write_u32(&mut self, value: u32) {
        self.write_u64(u64::from(value));
    }
    fn write_u64(&mut self, value: u64) {
        self.0 = (self.0.rotate_left(5) ^ value).wrapping_mul(0x51_7c_c1_b7_27_22_0a_95);
    }
    fn write_usize(&mut self, value: usize) {
        self.write_u64(value as u64);
    }
    fn write_isize(&mut self, value: isize) {
        self.write_u64(value as u64);
    }
    fn write_i64(&mut self, value: i64) {
        self.write_u64(value as u64);
    }
}

type QuickSet = HashSet<Vec<Value>, std::hash::BuildHasherDefault<Quick>>;

/// Distinct key values in first-occurrence order; a hashed set past a few of them.
#[derive(Debug, Default)]
pub(crate) struct Keys {
    set: QuickSet,
    order: Vec<Vec<Value>>,
}

/// How many keys are compared one by one before they are hashed.
const LINEAR_KEYS: usize = 8;

impl Keys {
    fn contains(&self, key: &[Value]) -> bool {
        if self.order.len() <= LINEAR_KEYS {
            self.order.iter().any(|held| held.as_slice() == key)
        } else {
            self.set.contains(key)
        }
    }
    fn is_empty(&self) -> bool {
        self.order.is_empty()
    }
    fn insert(&mut self, key: &[Value]) {
        self.order.push(key.to_vec());
        if self.order.len() > LINEAR_KEYS {
            if self.set.is_empty() {
                self.set.extend(self.order.iter().cloned());
            } else {
                self.set.insert(key.to_vec());
            }
        }
    }
}

const DEPTH: u32 = 48;

thread_local! {
    static REQUESTS: RefCell<Vec<(u32, u32)>> = const { RefCell::new(Vec::new()) };
    static COSTS: RefCell<Vec<[u64; 4]>> = const { RefCell::new(Vec::new()) };
    static WASTE: RefCell<Vec<[u32; 2]>> = const { RefCell::new(Vec::new()) };
    static PENDING: RefCell<(u32, Vec<u32>)> = const { RefCell::new((u32::MAX, Vec::new())) };
    static OWNS: RefCell<Vec<StageOwn>> = const { RefCell::new(Vec::new()) };
    static OWN_INDEX: RefCell<std::collections::HashMap<(usize, u32), &'static Lowering>> = RefCell::new(std::collections::HashMap::new());
}

/// The distinct keys of `rows` on `on`, at most `cap`; none past it.
fn keys_of(rows: InputRows<'_>, on: &[ColumnIndex], cap: usize) -> Option<Keys> {
    let mut keys = Keys::default();
    let mut key = Vec::with_capacity(on.len());
    for row in 0..count_of(rows).0 {
        key.clear();
        for column in on {
            key.push(cell_of(rows, RowIndex(row), *column)?);
        }
        if !keys.contains(&key) {
            if keys.order.len() >= cap {
                return None;
            }
            keys.insert(&key);
        }
    }
    Some(keys)
}

/// The rows of `rows` whose `on` cells are one of `keys`, in order, appended to `out` (its kinds set from `rows`).
fn filtered(rows: InputRows<'_>, on: &[ColumnIndex], keys: &Keys, scratch: &mut Vec<Value>, out: &mut Table) {
    out.kinds.clear();
    out.kinds.extend_from_slice(kinds_of(rows));
    let mut key = Vec::with_capacity(on.len());
    let mut count = 0;
    'rows: for row in 0..count_of(rows).0 {
        key.clear();
        for column in on {
            match cell_of(rows, RowIndex(row), *column) {
                Some(cell) => key.push(cell),
                None => continue 'rows,
            }
        }
        if keys.contains(&key) {
            out.cells.extend_from_slice(row_of(rows, RowIndex(row), scratch));
            count += 1;
        }
    }
    out.count = RowCount(count);
}

/// Counts a restricted request for `node` in the open file; answers how many there were before it.
fn asked(buffers: &FileBuffers, node: PlanNodeId) -> u32 {
    let stamp = buffers.generation;
    REQUESTS.with(|requests| {
        let mut requests = requests.borrow_mut();
        let at = node.0 as usize;
        if requests.len() <= at {
            requests.resize(at + 1, (u32::MAX, 0));
        }
        let slot = &mut requests[at];
        if slot.0 != stamp {
            *slot = (stamp, 0);
        }
        slot.1 += 1;
        slot.1 - 1
    })
}

fn kind_name(lowering: &Lowering) -> &'static str {
    match lowering {
        Lowering::Filter { .. } => "filter",
        Lowering::Lookup { .. } => "lookup",
        Lowering::Project { .. } => "project",
        Lowering::Extend { .. } => "extend",
        Lowering::Probe { .. } => "probe",
        Lowering::Distinct { .. } => "distinct",
        Lowering::Group { .. } => "group",
        Lowering::Reach { .. } => "reach",
        Lowering::SeededReach { .. } => "seeded_reach",
        Lowering::Axis { .. } => "axis",
        Lowering::OrderedWalk { .. } => "ordered_walk",
        Lowering::Worklist { .. } => "worklist",
        Lowering::Components { .. } => "components",
        Lowering::TextMap { .. } => "text_map",
        Lowering::OrderedMatch { .. } => "ordered_match",
        Lowering::Render { .. } => "render",
        Lowering::ScanBound { .. } => "scan_bound",
        Lowering::Unlowered => "unlowered",
    }
}

/// The context of one restricted request.
pub(crate) struct Bound<'a, 'k, 'r> {
    pub(crate) plan: &'a PhysicalPlan,
    pub(crate) run: &'k mut KernelRun<'r>,
    pub(crate) buffers: &'k mut FileBuffers,
}

impl Bound<'_, '_, '_> {
    /// `node` made in full and its rows with a key in `keys` copied to `out`; false on a pending fault.
    fn full(&mut self, node: PlanNodeId, on: &[ColumnIndex], keys: &Keys, out: &mut Table) -> bool {
        let started = std::time::Instant::now();
        let made = super::super::service::run(self.plan, self.run, node, self.buffers);
        crate::spike::bound_time(1, started);
        if made.is_err() {
            return false;
        }
        self.held(node, on, keys, out)
    }

    /// A node the open file holds or knows empty, filtered by `keys` into `out`; false when it holds a pending fault or
    /// neither.
    fn held(&mut self, node: PlanNodeId, on: &[ColumnIndex], keys: &Keys, out: &mut Table) -> bool {
        let place = node.0 as usize;
        if self.buffers.pending.get(place).is_some_and(Option::is_some) {
            return false;
        }
        let mut scratch = std::mem::take(&mut self.buffers.row);
        let answer = match self.buffers.rows(node) {
            NodeRows::Materialized(table) => {
                filtered(InputRows::Table(table), on, keys, &mut scratch, out);
                true
            }
            NodeRows::Absent if known(self.buffers, node.0) == Known::Empty => self.empty(node, out),
            NodeRows::Absent => false,
        };
        self.buffers.row = scratch;
        answer
    }

    /// `out` holds no row, in `node`'s planned layout.
    fn empty(&self, node: PlanNodeId, out: &mut Table) -> bool {
        match self.plan.layouts.get(node.0 as usize) {
            Some(Some(kinds)) => {
                out.kinds.clear();
                out.kinds.extend_from_slice(kinds);
                out.cells.clear();
                out.count = RowCount(0);
                true
            }
            _ => false,
        }
    }

    /// The rows of `source`, as `reader` reads it, whose `on` cells hold one of `keys`, into `out`.
    fn source(&mut self, reader: PlanNodeId, source: RowSource, on: &[ColumnIndex], keys: &Keys, out: &mut Table, depth: u32) -> bool {
        let mut scratch = std::mem::take(&mut self.buffers.row);
        let answer = match source {
            RowSource::Node(node) => {
                self.buffers.row = scratch;
                return self.restricted(node, on, keys, out, depth + 1);
            }
            RowSource::Facts(relation) => match facts(self.run.inputs, relation) {
                Ok(view) if keys.is_empty() => {
                    out.kinds.clear();
                    out.kinds.extend_from_slice(view.kinds);
                    true
                }
                Ok(view) => {
                    filtered(InputRows::Facts(&view), on, keys, &mut scratch, out);
                    true
                }
                Err(_) => false,
            },
            RowSource::Tree(relation) => {
                let roots = rooted(self.run.inputs, reader, relation);
                let tree = tree_read(roots, self.run.resources.tree());
                filtered(InputRows::Tree(&tree), on, keys, &mut scratch, out);
                true
            }
            RowSource::Frontier(_) | RowSource::Observation(_) => false,
        };
        self.buffers.row = scratch;
        answer
    }

    /// The rows of `node` whose `on` columns hold one of `keys`, as a bag and in the node's order, into `out`. False when
    /// they could not be made (a fault on the way): the caller makes the node in full.
    pub(crate) fn restricted(&mut self, node: PlanNodeId, on: &[ColumnIndex], keys: &Keys, out: &mut Table, depth: u32) -> bool {
        out.cells.clear();
        out.count = RowCount(0);
        super::super::service::sized(self.plan, self.buffers);
        let place = node.0 as usize;
        if self.buffers.rows(node) != NodeRows::Absent || known(self.buffers, node.0) == Known::Empty {
            return self.held(node, on, keys, out);
        }
        // No key: no row, as the lazy path answers a node over no row, without making anything under it.
        if keys.is_empty() && self.empty(node, out) {
            return true;
        }
        let Some(physical) = self.plan.nodes.get(place) else {
            return false;
        };
        let lowering = &physical.lowering;
        if depth > DEPTH
            || (depth > 0
                && (physical.readers.0 as usize > crate::spike::bound_knob(2)
                    || asked(self.buffers, node) as usize + 1 >= crate::spike::bound_knob(1)))
        {
            return self.full(node, on, keys, out);
        }
        if guarded_empty(self.plan, self.run, node, self.buffers) {
            marked_empty(self.plan, self.buffers, node);
            return self.empty(node, out);
        }
        if !crate::spike::entity_off()
            && self.plan.entity_queries.exported(crate::domain::entity_queries::types::EntityExportId(node.0)).is_some()
        {
            return self.fell(node, lowering, on, keys, out);
        }
        match lowering {
            Lowering::Filter { source, select } => {
                let mapped: Option<Vec<ColumnIndex>> = if select.kept.is_empty() {
                    Some(on.to_vec())
                } else {
                    on.iter().map(|column| select.kept.get(column.0 as usize).copied()).collect()
                };
                let Some(mapped) = mapped else { return self.fell(node, lowering, on, keys, out) };
                self.row_wise(node, lowering, *source, (on, &mapped), keys, out, depth)
            }
            Lowering::Project { source, project } => {
                let mapped: Option<Vec<ColumnIndex>> =
                    on.iter().map(|column| project.columns.get(column.0 as usize).copied()).collect();
                let Some(mapped) = mapped else { return self.fell(node, lowering, on, keys, out) };
                self.row_wise(node, lowering, *source, (on, &mapped), keys, out, depth)
            }
            Lowering::Extend { source, extension } => {
                let width = self.plan.layouts.get(place).and_then(Option::as_ref).map(Vec::len);
                let Some(width) = width else { return self.fell(node, lowering, on, keys, out) };
                let carried = width.saturating_sub(extension.values.len());
                if on.iter().any(|column| column.0 as usize >= carried) {
                    return self.fell(node, lowering, on, keys, out);
                }
                self.row_wise(node, lowering, *source, (on, on), keys, out, depth)
            }
            Lowering::TextMap { source, map } => {
                let width = self.plan.layouts.get(place).and_then(Option::as_ref).map(Vec::len);
                let Some(width) = width else { return self.fell(node, lowering, on, keys, out) };
                let carried = width.saturating_sub(
                    crate::domain::physical_plan::collections::kernels::text_map::item::mapped_kinds(map, &[]).len(),
                );
                if on.iter().any(|column| column.0 as usize >= carried) {
                    return self.fell(node, lowering, on, keys, out);
                }
                self.row_wise(node, lowering, *source, (on, on), keys, out, depth)
            }
            Lowering::Probe { left, right, probe, index } => {
                if !crate::spike::bound_shape(1) {
                    return self.fell(node, lowering, on, keys, out);
                }
                self.probe(node, lowering, (*left, *right, probe, *index), on, keys, out, depth)
            }
            Lowering::Distinct { sources: [first, second] } => {
                if !crate::spike::bound_shape(2) {
                    return self.fell(node, lowering, on, keys, out);
                }
                let mut a = self.buffers.pool.fresh();
                let mut b = self.buffers.pool.fresh();
                clear(&mut a);
                clear(&mut b);
                let made = self.source(node, *first, on, keys, &mut a, depth)
                    && self.source(node, *second, on, keys, &mut b, depth)
                    && self.held_kernel(node, lowering, &a, Some(&b), out);
                self.buffers.pool.pooled(a, PoolSize(POOL_TABLES));
                self.buffers.pool.pooled(b, PoolSize(POOL_TABLES));
                made || self.full(node, on, keys, out)
            }
            Lowering::Group { source, group } => {
                if !crate::spike::bound_shape(2) {
                    return self.fell(node, lowering, on, keys, out);
                }
                let mapped: Option<Vec<ColumnIndex>> =
                    on.iter().map(|column| group.keys.get(column.0 as usize).copied()).collect();
                let Some(mapped) = mapped else { return self.fell(node, lowering, on, keys, out) };
                let mut input = self.buffers.pool.fresh();
                clear(&mut input);
                let made = self.source(node, *source, &mapped, keys, &mut input, depth)
                    && self.held_kernel(node, lowering, &input, None, out);
                self.buffers.pool.pooled(input, PoolSize(POOL_TABLES));
                made || self.full(node, on, keys, out)
            }
            _ => self.fell(node, lowering, on, keys, out),
        }
    }

    /// A node the restriction does not reach: made in full and filtered.
    fn fell(&mut self, node: PlanNodeId, lowering: &Lowering, on: &[ColumnIndex], keys: &Keys, out: &mut Table) -> bool {
        crate::spike::bound_fell(kind_name(lowering));
        self.full(node, on, keys, out)
    }

    /// A row-wise node over its restricted source.
    #[expect(clippy::too_many_arguments, reason = "spike")]
    fn row_wise(&mut self, node: PlanNodeId, lowering: &Lowering, source: RowSource, (on, mapped): (&[ColumnIndex], &[ColumnIndex]), keys: &Keys, out: &mut Table, depth: u32) -> bool {
        let mut input = self.buffers.pool.fresh();
        clear(&mut input);
        let made = self.source(node, source, mapped, keys, &mut input, depth)
            && self.streamed(node, lowering, InputRows::Table(&input), None, out);
        self.buffers.pool.pooled(input, PoolSize(POOL_TABLES));
        made || self.full(node, on, keys, out)
    }

    /// A probe restricted on columns of one of its sides.
    #[expect(clippy::too_many_arguments, reason = "spike")]
    fn probe(
        &mut self,
        node: PlanNodeId,
        lowering: &Lowering,
        (left, right, probe, index): (RowSource, RowSource, &ProbeParts, ProbeIndex),
        on: &[ColumnIndex],
        keys: &Keys,
        out: &mut Table,
        depth: u32,
    ) -> bool {
        let (left_key, right_key): (Vec<ColumnIndex>, Option<Vec<ColumnIndex>>) = match &probe.key {
            ProbeKey::Columns { left, right } => (left.clone(), Some(right.clone())),
            ProbeKey::Whole => {
                crate::spike::bound_fell("probe/whole-key");
                return self.fell(node, lowering, on, keys, out);
            }
        };
        let right_key = right_key.unwrap_or_default();
        if index == ProbeIndex::Tree || matches!(probe.mode, ProbeMode::Ordered(_)) || left == right {
            crate::spike::bound_fell(if index == ProbeIndex::Tree {
                "probe/tree"
            } else if left == right {
                "probe/self"
            } else {
                "probe/ordered"
            });
            return self.fell(node, lowering, on, keys, out);
        }
        // The columns `on` names, on the side they come from.
        let sides: Option<Vec<(Side, ColumnIndex)>> = if probe.mode == ProbeMode::Inner {
            on.iter().map(|column| probe.output.get(column.0 as usize).map(|output| (output.side, output.column))).collect()
        } else {
            Some(on.iter().map(|column| (Side::Left, *column)).collect())
        };
        let Some(sides) = sides else {
            crate::spike::bound_fell("probe/no-column");
            return self.fell(node, lowering, on, keys, out);
        };
        let mut left_rows = self.buffers.pool.fresh();
        let mut right_rows = self.buffers.pool.fresh();
        clear(&mut left_rows);
        clear(&mut right_rows);
        let made = if sides.iter().all(|(side, _)| *side == Side::Left) {
            let mapped: Vec<ColumnIndex> = sides.iter().map(|(_, column)| *column).collect();
            self.source(node, left, &mapped, keys, &mut left_rows, depth)
                && (left_rows.count.0 == 0 && self.empty(node, out)
                    || self.probe_right(node, right, &left_rows, &left_key, &right_key, &mut right_rows, depth)
                        && !(right_rows.count.0 == 0 && probe.mode != ProbeMode::Anti && self.empty(node, out))
                    && self.streamed(node, own_index(self.plan, node, lowering), InputRows::Table(&left_rows), Some(&ChainRows::Held(&right_rows)), out))
        } else if probe.mode == ProbeMode::Inner && sides.iter().all(|(side, _)| *side == Side::Right) {
            let mapped: Vec<ColumnIndex> = sides.iter().map(|(_, column)| *column).collect();
            self.source(node, right, &mapped, keys, &mut right_rows, depth)
                && (right_rows.count.0 == 0 && self.empty(node, out) || match keys_of(InputRows::Table(&right_rows), &right_key, usize::MAX) {
                    Some(left_keys) => self.source(node, left, &left_key, &left_keys, &mut left_rows, depth),
                    None => false,
                }
                && self.streamed(node, own_index(self.plan, node, lowering), InputRows::Table(&left_rows), Some(&ChainRows::Held(&right_rows)), out))
        } else {
            self.buffers.pool.pooled(left_rows, PoolSize(POOL_TABLES));
            self.buffers.pool.pooled(right_rows, PoolSize(POOL_TABLES));
            crate::spike::bound_fell("probe/mixed-sides");
            return self.fell(node, lowering, on, keys, out);
        };
        self.buffers.pool.pooled(left_rows, PoolSize(POOL_TABLES));
        self.buffers.pool.pooled(right_rows, PoolSize(POOL_TABLES));
        made || self.full(node, on, keys, out)
    }

    /// A probe's right rows for the left rows `left`: restricted to their keys, or the right side in full.
    #[expect(clippy::too_many_arguments, reason = "spike")]
    fn probe_right(&mut self, reader: PlanNodeId, right: RowSource, left: &Table, left_key: &[ColumnIndex], right_key: &[ColumnIndex], out: &mut Table, depth: u32) -> bool {
        match keys_of(InputRows::Table(left), left_key, usize::MAX) {
            Some(keys) => self.source(reader, right, right_key, &keys, out, depth),
            None => false,
        }
    }

    /// Runs a streamed lowering's kernel over `input` (and a probe's right rows) into `out`; false on any fault.
    fn streamed(&mut self, node: PlanNodeId, lowering: &Lowering, input: InputRows<'_>, right: Option<&ChainRows<'_>>, out: &mut Table) -> bool {
        let stage = ChainStage { node, lowering };
        let mut own = OWNS.with(|owns| owns.borrow_mut().pop()).unwrap_or_default();
        clear(out);
        let answer = (|| {
            let prepared = stage_prepared(self.run, self.buffers, stage, kinds_of(input), &mut own, right).ok()?;
            if prepared.fault.is_some() {
                return None;
            }
            out.kinds = prepared.kinds;
            let mut from = RowIndex(0);
            loop {
                let read = stage_chunk(self.run, self.buffers, stage, RowChunk { rows: input, from }, &mut own, right, out, CellBound(usize::MAX)).ok()?;
                match read.stop {
                    ChunkStop::Ended => return Some(()),
                    ChunkStop::Full => from = RowIndex(from.0 + read.read.0),
                    ChunkStop::Faulted(_) | ChunkStop::Hard(_) => return None,
                }
                if from.0 >= count_of(input).0 {
                    return Some(());
                }
            }
        })()
        .is_some();
        own.work = crate::domain::physical_plan::types::stream_scratch::value::ProbeWork::Pending;
        OWNS.with(|owns| owns.borrow_mut().push(own));
        answer
    }

    /// Runs a held lowering's kernel (distinct, group) over tables into `out`; false on any fault.
    fn held_kernel(&mut self, node: PlanNodeId, lowering: &Lowering, input: &Table, other: Option<&Table>, out: &mut Table) -> bool {
        clear(out);
        let inputs = KernelInputs { input: InputRows::Table(input), other: other.map(InputRows::Table), third: None };
        matches!(kernels(lowering, self.run, node, inputs, self.buffers, out), Ok(None))
    }
}

fn clear(table: &mut Table) {
    table.cells.clear();
    table.kinds.clear();
    table.count = RowCount(0);
}

/// `lowering` as a probe of its own index: a probe of a shared index reads restricted right rows through an index of its own.
pub(crate) fn own_index<'a>(plan: &PhysicalPlan, node: PlanNodeId, lowering: &'a Lowering) -> &'a Lowering {
    let Lowering::Probe { index: ProbeIndex::Shared(_), .. } = lowering else {
        return lowering;
    };
    let key = (std::ptr::from_ref(plan) as usize, node.0);
    OWN_INDEX.with(|cache| {
        *cache.borrow_mut().entry(key).or_insert_with(|| {
            let mut own = lowering.clone();
            if let Lowering::Probe { index, .. } = &mut own {
                *index = ProbeIndex::Own;
            }
            Box::leak(Box::new(own))
        })
    })
}

/// Spike (ryo-22c7): the top-level request: probe `probe_node` reads `source` on its left and the unmade node `right`.
/// Answers the rows of `right` whose right key holds one of the left's key values, when the left holds few of them.
#[expect(clippy::too_many_arguments, reason = "spike")]
pub(crate) fn bounded(
    plan: &PhysicalPlan,
    run: &mut KernelRun<'_>,
    probe_node: PlanNodeId,
    probe: &ProbeParts,
    index: ProbeIndex,
    source: RowSource,
    right: PlanNodeId,
    buffers: &mut FileBuffers,
) -> Option<Table> {
    if index == ProbeIndex::Tree || matches!(probe.mode, ProbeMode::Ordered(_)) || source == RowSource::Node(right) {
        return None;
    }
    let mode = match probe.mode {
        ProbeMode::Inner => 1,
        ProbeMode::Semi => 2,
        ProbeMode::Anti | ProbeMode::Ordered(_) => 4,
    };
    if crate::spike::bound_knob(7) & mode == 0 {
        return None;
    }
    let ProbeKey::Columns { left: left_key, right: right_key } = &probe.key else {
        return None;
    };
    if buffers.rows(right) != NodeRows::Absent || known(buffers, right.0) != Known::Unmade {
        return None;
    }
    settle_waste(buffers);
    if !chosen(right) || wasteful(right) {
        return None;
    }
    let keys = {
        let RowSource::Node(read) = source else {
            return None;
        };
        if buffers.pending.get(read.0 as usize).is_some_and(Option::is_some) {
            return None;
        }
        let NodeRows::Materialized(table) = buffers.rows(read) else {
            return None;
        };
        keys_of(InputRows::Table(table), left_key, crate::spike::bound_knob(0))?
    };
    let _ = probe_node;
    if plan.nodes.get(right.0 as usize).is_none_or(|node| node.readers.0 as usize > crate::spike::bound_knob(3)) {
        return None;
    }
    if asked(buffers, right) as usize + 1 >= crate::spike::bound_knob(1) {
        return None;
    }
    let mut out = buffers.pool.fresh();
    clear(&mut out);
    let started = std::time::Instant::now();
    let made = Bound { plan, run, buffers }.restricted(right, right_key, &keys, &mut out, 0);
    crate::spike::bound_time(0, started);
    if made {
        cost_add(right, 2, u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX));
        restricted_now(right);
    }
    if !made {
        buffers.pool.pooled(out, PoolSize(POOL_TABLES));
        return None;
    }
    if crate::spike::bound_check() {
        checked(plan, run, right, right_key, &keys, &out, buffers);
    }
    crate::spike::bound_served();
    Some(out)
}

/// The self-check: `right` made in full and filtered by `keys` must equal `out`, row for row.
fn checked(plan: &PhysicalPlan, run: &mut KernelRun<'_>, right: PlanNodeId, on: &[ColumnIndex], keys: &Keys, out: &Table, buffers: &mut FileBuffers) {
    if super::super::service::run(plan, run, right, buffers).is_err() {
        eprintln!("bound check: node {} faulted in full", right.0);
        return;
    }
    let mut full = Table::default();
    let mut bound = Bound { plan, run, buffers };
    if !bound.held(right, on, keys, &mut full) {
        eprintln!("bound check: node {} holds a pending fault in full", right.0);
        return;
    }
    let same_kinds = full.kinds == out.kinds;
    if !same_kinds || full.count != out.count || full.cells != out.cells {
        let first = full.cells.iter().zip(out.cells.iter()).position(|(a, b)| a != b);
        eprintln!(
            "bound check MISMATCH at node {}: full {} rows kinds {:?}, restricted {} rows kinds {:?}, first differing cell {:?}; keys {:?}; lowering {:.600?}",
            right.0, full.count.0, full.kinds, out.count.0, out.kinds, first, keys.order, plan.nodes.get(right.0 as usize).map(|node| &node.lowering)
        );
        std::process::abort();
    }
}

/// Spike (ryo-22c7): what this worker learned of node `node`: [nanos made in full, times, nanos restricted, times].
fn cost_of(node: PlanNodeId) -> [u64; 4] {
    COSTS.with(|costs| costs.borrow().get(node.0 as usize).copied().unwrap_or_default())
}

static GLOBAL_COSTS: std::sync::Mutex<Vec<[u64; 4]>> = std::sync::Mutex::new(Vec::new());

/// Spike (ryo-22c7): the run's per-node costs, the nodes with the most time first, as JSON lines.
pub(crate) fn costs_text() -> String {
    let costs = GLOBAL_COSTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
    let mut rows: Vec<(usize, [u64; 4])> = costs.iter().copied().enumerate().filter(|(_, cost)| cost[1] + cost[3] > 0).collect();
    rows.sort_by_key(|(_, cost)| std::cmp::Reverse(cost[0] + cost[2]));
    rows.iter()
        .take(40)
        .map(|(node, [full, fulls, bound, bounds])| {
            format!(
                "{{\"node\":{node},\"fulls\":{fulls},\"full_us\":{:.1},\"bounds\":{bounds},\"bound_us\":{:.1}}}\n",
                *full as f64 / 1e3 / (*fulls).max(1) as f64,
                *bound as f64 / 1e3 / (*bounds).max(1) as f64
            )
        })
        .collect()
}

fn cost_add(node: PlanNodeId, at: usize, nanos: u64) {
    static REPORTED: std::sync::OnceLock<bool> = std::sync::OnceLock::new();
    if *REPORTED.get_or_init(|| std::env::var_os("RYO_BOUND_REPORT").is_some()) {
        let mut costs = GLOBAL_COSTS.lock().unwrap_or_else(std::sync::PoisonError::into_inner);
        let place = node.0 as usize;
        if costs.len() <= place {
            costs.resize(place + 1, [0; 4]);
        }
        if let Some(slot) = costs.get_mut(place) {
            slot[at] += nanos;
            slot[at + 1] += 1;
        }
    }
    COSTS.with(|costs| {
        let mut costs = costs.borrow_mut();
        let place = node.0 as usize;
        if costs.len() <= place {
            costs.resize(place + 1, [0; 4]);
        }
        if let Some(slot) = costs.get_mut(place) {
            slot[at] += nanos;
            slot[at + 1] += 1;
        }
    });
}

/// Spike (ryo-22c7): a probe's unmade right side `node` was made in full, inclusive of what it made first, in `nanos`.
pub(crate) fn full_cost(node: PlanNodeId, nanos: u64) {
    cost_add(node, 0, nanos);
}

/// Whether a request for `node` is restricted: each way is tried `RYO_BOUND_LEARN` times (default 2) on this worker, then
/// restriction is kept where its mean cost is under `RYO_BOUND_RATIO` percent (default 80) of the full make's.
fn chosen(node: PlanNodeId) -> bool {
    let [full, fulls, bound, bounds] = cost_of(node);
    let learn = crate::spike::bound_knob(4) as u64;
    if fulls < learn {
        return false;
    }
    if bounds < learn {
        return true;
    }
    // bound / bounds < ratio * full / fulls
    u128::from(bound) * u128::from(fulls) * 100 < u128::from(full) * u128::from(bounds) * crate::spike::bound_knob(5) as u128
}

/// Spike (ryo-22c7): the nodes restricted in the file before this one that the file then made in full anyway count as wasted.
fn settle_waste(buffers: &FileBuffers) {
    PENDING.with(|pending| {
        let (generation, nodes) = &mut *pending.borrow_mut();
        if *generation == buffers.generation {
            return;
        }
        let stamp = generation.wrapping_add(1);
        WASTE.with(|waste| {
            let mut waste = waste.borrow_mut();
            for node in nodes.iter() {
                let place = *node as usize;
                if waste.len() <= place {
                    waste.resize(place + 1, [0; 2]);
                }
                let made = buffers.known_full.get(place) == Some(&stamp) || buffers.known_empty.get(place) == Some(&stamp);
                if let Some(slot) = waste.get_mut(place) {
                    slot[0] += 1;
                    if made {
                        slot[1] += 1;
                    }
                }
            }
        });
        nodes.clear();
        *generation = buffers.generation;
    });
}

fn restricted_now(node: PlanNodeId) {
    PENDING.with(|pending| pending.borrow_mut().1.push(node.0));
}

/// Whether restriction of `node` was mostly wasted on this worker: `RYO_BOUND_WASTE` percent (default 34) of its files made it
/// in full anyway.
fn wasteful(node: PlanNodeId) -> bool {
    WASTE.with(|waste| {
        waste.borrow().get(node.0 as usize).is_some_and(|[seen, made]| {
            *seen >= 2 && u64::from(*made) * 100 > u64::from(*seen) * crate::spike::bound_knob(6) as u64
        })
    })
}
