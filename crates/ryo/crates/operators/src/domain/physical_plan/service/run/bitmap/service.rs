//! Spike (ryo-bshs): the existence pre-pass over key sets. Each node's answer is abstracted, per column, as a set of the
//! open file's ids that holds every value the column can hold (an over-approximation): a select over a relation sets the bits
//! of the rows that pass it, a keyed join intersects its key columns' sets, a union joins them, a projection moves them; what
//! the pass cannot follow keeps its input's sets or none at all ("any value"). A node with a column whose set is empty holds
//! no row. The pass may answer "maybe" where the node is empty, never "empty" where it holds a row.
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::equality_join::types::{ProbeKey, ProbeMode};
use crate::domain::comparing::service::fast_holds;
use crate::domain::physical_plan::collections::kernels::filter::item::{FAST, filter, holds, readied};
use crate::domain::physical_plan::collections::kernels::scan_facts::item::{decode, facts, facts_cell_at, rows_of};
use crate::domain::physical_plan::collections::kernels::scan_roots::item::{roots, tree_count, tree_row};
use crate::domain::physical_plan::types::file_buffers::value::FileBuffers;
use crate::domain::physical_plan::types::lowering::value::Lowering;
use crate::domain::physical_plan::types::physical_plan::value::PhysicalPlan;
use crate::domain::physical_plan::types::{KernelRun, PlanNodeId, RowSource};
use crate::domain::select::types::SelectParts;
use ryo_relations::domain::relations::types::{CellKind, Value};
use ryo_values::domain::plan_parts::types::{ColumnIndex, Operand, Side, Test};
use ryo_values::domain::rows::types::RowIndex;

/// A column that may hold any value.
const TOP: u32 = u32::MAX;
/// The most words a set has.
const MOST_WORDS: usize = 1024;
/// The fewest words a set has.
const FEWEST_WORDS: usize = 4;

/// Where a set's places are.
#[derive(Clone, Copy, Debug, Default)]
struct SetMeta {
    /// Dense: `words` words from `start` in `dense`; else `len` places from `start` in `items`.
    dense: bool,
    start: u32,
    len: u32,
}

/// One node's abstract answer in the open file.
#[derive(Clone, Copy, Debug, Default)]
struct Val {
    /// Proved to hold no row.
    empty: bool,
    /// Every column may hold any value (the width is not followed).
    top: bool,
    /// The first of its columns' sets in `cols`.
    start: u32,
    /// How many columns.
    len: u32,
}

/// Spike (ryo-bshs): the pass's per-worker state, its sets cleared for each file.
#[derive(Debug, Default)]
pub(crate) struct BitmapScratch {
    /// The file the sets below belong to (the buffers' stamp).
    file: u32,
    /// Words per set in the open file.
    words: usize,
    /// Mask of a bit's place.
    mask: u32,
    /// Shift of a hashed cell to its place.
    shift: u32,
    /// By node, the file whose answer `val` holds.
    stamp: Vec<u32>,
    /// By node, its answer's place in `vals`.
    val: Vec<u32>,
    /// The answers of the open file.
    vals: Vec<Val>,
    /// The answers' column sets: a set's number in `sets`, or `TOP`.
    cols: Vec<u32>,
    /// By entry of `cols`, whether the set holds exactly the places of the column's values, one value per place.
    exact: Vec<bool>,
    /// The sets of the open file: dense (`words` words in `dense` from `start`) or a sorted list of places (`items`).
    sets: Vec<SetMeta>,
    /// The dense sets' words.
    dense: Vec<u64>,
    /// The listed sets' places.
    items: Vec<u32>,
    /// Sets being built: their places, unsorted.
    build: Vec<Vec<u32>>,
    /// How many of `build` are open.
    open: usize,
    /// A decoded row.
    row: Vec<Value>,
    /// By node, the file in which its answer was read from its made table.
    tabled: Vec<u32>,
    /// By relation number, the file whose whole-relation answer `rel_val` holds.
    rel_stamp: Vec<u32>,
    /// By relation number, the answer of all its rows.
    rel_val: Vec<u32>,
    /// By node, across this worker's files: how often the pass was asked for it and how often it proved it empty.
    tries: Vec<(u16, u16)>,
    /// By node, the file in which it is on the evaluation stack, so a cycle answers "maybe".
    busy: Vec<u32>,
}

impl BitmapScratch {
    fn begin(&mut self, stamp: u32, elements: usize) {
        if self.file == stamp {
            return;
        }
        self.file = stamp;
        let bits = elements.max(1).next_power_of_two();
        self.words = (bits / 64).clamp(FEWEST_WORDS, MOST_WORDS);
        let places = self.words * 64;
        self.mask = u32::try_from(places - 1).unwrap_or(u32::MAX);
        self.shift = 64 - places.trailing_zeros();
        self.vals.clear();
        self.cols.clear();
        self.exact.clear();
        self.sets.clear();
        self.dense.clear();
        self.items.clear();
        self.open = 0;
    }

    /// Opens a set to build; its handle is a number below `TOP` until `close`.
    fn fresh(&mut self) -> u32 {
        if self.build.len() <= self.open {
            self.build.push(Vec::new());
        }
        self.build[self.open].clear();
        self.open += 1;
        u32::try_from(self.open - 1).unwrap_or(0)
    }

    /// Adds a place to an open set.
    #[inline]
    fn add(&mut self, handle: u32, place: usize) {
        self.build[handle as usize].push(place as u32);
    }

    /// Closes every open set, in `cols` (entries that are `TOP` stay so), into stored sets.
    fn close_all(&mut self, cols: &mut [u32]) {
        for col in cols.iter_mut() {
            if *col != TOP {
                let handle = *col as usize;
                let mut places = std::mem::take(&mut self.build[handle]);
                *col = self.stored(&mut places);
                places.clear();
                self.build[handle] = places;
            }
        }
        self.open = 0;
    }

    /// A stored set of `places` (any order, repeats allowed).
    fn stored(&mut self, places: &mut Vec<u32>) -> u32 {
        let id = u32::try_from(self.sets.len()).unwrap_or(TOP - 1);
        if places.len() > 2 * self.words {
            let start = self.dense.len();
            self.dense.resize(start + self.words, 0);
            for place in places.iter() {
                let place = *place as usize;
                self.dense[start + place / 64] |= 1 << (place % 64);
            }
            self.sets.push(SetMeta { dense: true, start: start as u32, len: 0 });
        } else {
            places.sort_unstable();
            places.dedup();
            let start = self.items.len();
            self.items.extend_from_slice(places);
            self.sets.push(SetMeta { dense: false, start: start as u32, len: places.len() as u32 });
        }
        id
    }

    fn list(&self, id: u32) -> &[u32] {
        let meta = self.sets[id as usize];
        &self.items[meta.start as usize..(meta.start + meta.len) as usize]
    }

    fn words_of(&self, id: u32) -> &[u64] {
        let meta = self.sets[id as usize];
        &self.dense[meta.start as usize..meta.start as usize + self.words]
    }

    /// Whether set `id` holds `place`.
    #[inline]
    fn contains(&self, id: u32, place: usize) -> bool {
        if id == TOP {
            return true;
        }
        if self.sets[id as usize].dense {
            self.words_of(id)[place / 64] & (1 << (place % 64)) != 0
        } else {
            self.list(id).binary_search(&(place as u32)).is_ok()
        }
    }

    fn zero(&self, id: u32) -> bool {
        if id == TOP {
            return false;
        }
        if self.sets[id as usize].dense { self.words_of(id).iter().all(|word| *word == 0) } else { self.sets[id as usize].len == 0 }
    }

    /// A stored list from the scratch `places` (sorted, no repeats).
    fn listed(&mut self, places: &[u32]) -> u32 {
        let id = u32::try_from(self.sets.len()).unwrap_or(TOP - 1);
        let start = self.items.len();
        self.items.extend_from_slice(places);
        self.sets.push(SetMeta { dense: false, start: start as u32, len: places.len() as u32 });
        id
    }

    /// A stored dense set from words computed by `word(at)`.
    fn densed(&mut self, word: impl Fn(&Self, usize) -> u64) -> u32 {
        let mut words: Vec<u64> = (0..self.words).map(|at| word(self, at)).collect();
        let ones: u32 = words.iter().map(|word| word.count_ones()).sum();
        if (ones as usize) <= 2 * self.words {
            let mut places = Vec::with_capacity(ones as usize);
            for (at, word) in words.iter().enumerate() {
                let mut bits = *word;
                while bits != 0 {
                    places.push((at * 64) as u32 + bits.trailing_zeros());
                    bits &= bits - 1;
                }
            }
            return self.listed(&places);
        }
        let id = u32::try_from(self.sets.len()).unwrap_or(TOP - 1);
        let start = self.dense.len();
        self.dense.append(&mut words);
        self.sets.push(SetMeta { dense: true, start: start as u32, len: 0 });
        id
    }

    /// The places of list `one` that `keep` accepts.
    fn filtered(&mut self, one: u32, keep: impl Fn(&Self, u32) -> bool) -> u32 {
        let places: Vec<u32> = self.list(one).iter().copied().filter(|place| keep(self, *place)).collect();
        self.listed(&places)
    }

    /// The intersection of two columns' sets.
    fn and(&mut self, one: u32, other: u32) -> u32 {
        if one == TOP {
            return other;
        }
        if other == TOP || one == other {
            return one;
        }
        match (self.sets[one as usize].dense, self.sets[other as usize].dense) {
            (false, _) => self.filtered(one, |this, place| this.contains(other, place as usize)),
            (true, false) => self.filtered(other, |this, place| this.contains(one, place as usize)),
            (true, true) => self.densed(|this, at| this.words_of(one)[at] & this.words_of(other)[at]),
        }
    }

    /// The union of two columns' sets.
    fn or(&mut self, one: u32, other: u32) -> u32 {
        if one == TOP || other == TOP {
            return TOP;
        }
        if one == other {
            return one;
        }
        match (self.sets[one as usize].dense, self.sets[other as usize].dense) {
            (false, false) => {
                let mut places: Vec<u32> = self.list(one).iter().chain(self.list(other)).copied().collect();
                self.stored(&mut places)
            }
            _ => self.densed(|this, at| this.word_at(one, at) | this.word_at(other, at)),
        }
    }

    /// Word `at` of a set, a list spread into words.
    fn word_at(&self, id: u32, at: usize) -> u64 {
        if self.sets[id as usize].dense {
            return self.words_of(id)[at];
        }
        let list = self.list(id);
        let low = (at * 64) as u32;
        let from = list.partition_point(|place| *place < low);
        let mut word = 0u64;
        for place in &list[from..] {
            if *place >= low + 64 {
                break;
            }
            word |= 1 << (place - low);
        }
        word
    }

    /// The place of a cell of `kind` stored as `bits`.
    fn place(&self, kind: CellKind, bits: u64) -> usize {
        let place = match kind {
            CellKind::Node | CellKind::Symbol => (bits & 0xffff_ffff) as u32,
            CellKind::File | CellKind::Span | CellKind::Number | CellKind::Flag => {
                (bits.wrapping_mul(0x9e37_79b9_7f4a_7c15) >> self.shift) as u32
            }
        };
        (place & self.mask) as usize
    }

    /// The place of a cell; `None` for a cell whose equality is not its stored number (a name that left its file).
    fn place_of(&self, value: Value) -> Option<usize> {
        let kind = match value {
            Value::Node(_) => CellKind::Node,
            Value::Symbol(_) => CellKind::Symbol,
            Value::Name(_) => return None,
            Value::File(_) => CellKind::File,
            Value::Span(_) => CellKind::Span,
            Value::Number(_) => CellKind::Number,
            Value::Flag(_) => CellKind::Flag,
        };
        Some(self.place(kind, ryo_relations::domain::relations::service::to_bits(value).0))
    }

    fn answer(&mut self, empty: bool, top: bool, cols: &[u32]) -> u32 {
        self.answer_exact(empty, top, cols, &[])
    }

    /// An answer whose columns are exact where `exact` says so (missing entries are not exact).
    fn answer_exact(&mut self, empty: bool, top: bool, cols: &[u32], exact: &[bool]) -> u32 {
        self.open = 0;
        let start = u32::try_from(self.cols.len()).unwrap_or(u32::MAX);
        let empty = empty || cols.iter().any(|col| self.zero(*col));
        self.cols.extend_from_slice(cols);
        self.exact.extend((0..cols.len()).map(|at| exact.get(at).copied().unwrap_or(false) && cols[at] != TOP));
        let len = u32::try_from(cols.len()).unwrap_or(0);
        let id = u32::try_from(self.vals.len()).unwrap_or(u32::MAX);
        self.vals.push(Val { empty, top, start, len });
        id
    }

    fn maybe(&mut self) -> u32 {
        self.answer(false, true, &[])
    }

    fn nothing(&mut self) -> u32 {
        self.answer(true, true, &[])
    }

    fn col(&self, val: u32, column: usize) -> u32 {
        let val = self.vals[val as usize];
        if val.top || column >= val.len as usize {
            return TOP;
        }
        self.cols[val.start as usize + column]
    }

    fn exact_of(&self, val: u32) -> Vec<bool> {
        let val = self.vals[val as usize];
        if val.top {
            return Vec::new();
        }
        self.exact[val.start as usize..(val.start + val.len) as usize].to_vec()
    }

    /// The set of `one` less the places of `other`.
    fn less(&mut self, one: u32, other: u32) -> u32 {
        if self.sets[one as usize].dense {
            self.densed(|this, at| this.words_of(one)[at] & !this.word_at(other, at))
        } else {
            self.filtered(one, |this, place| !this.contains(other, place as usize))
        }
    }

    /// Whether a cell of `kind` stored as `bits` has a place of its own (no other value shares it).
    fn own_place(&self, kind: CellKind, bits: u64) -> bool {
        matches!(kind, CellKind::Node | CellKind::Symbol) && (bits & 0xffff_ffff) <= u64::from(self.mask)
    }

    fn cols_of(&self, val: u32) -> Vec<u32> {
        let val = self.vals[val as usize];
        if val.top {
            return Vec::new();
        }
        self.cols[val.start as usize..(val.start + val.len) as usize].to_vec()
    }

    /// The same answer with no column exact: what a kernel keeping some of its input's rows answers.
    fn inexact(&mut self, val: u32) -> u32 {
        let v = self.vals[val as usize];
        if v.empty || v.top {
            return val;
        }
        let cols = self.cols_of(val);
        self.answer(false, false, &cols)
    }

    fn is_empty(&self, val: u32) -> bool {
        self.vals[val as usize].empty
    }

    fn is_top(&self, val: u32) -> bool {
        self.vals[val as usize].top
    }
}

/// Spike (ryo-bshs): whether the pass proves `node` holds no row in the open file. Answers are kept for the rest of the file.
pub(crate) fn proved_empty(plan: &PhysicalPlan, run: &KernelRun<'_>, node: PlanNodeId, buffers: &mut FileBuffers) -> bool {
    let place = node.0 as usize;
    if buffers.bitmap.tries.len() <= place {
        buffers.bitmap.tries.resize(plan.nodes.len().max(place + 1), (0, 0));
    }
    // Only a node with an input still to make can save anything: a leaf or a node over made inputs runs its kernel anyway.
    let Some(physical) = plan.nodes.get(place) else { return false };
    let unmade = physical.lowering.sources().any(|source| match source {
        RowSource::Node(read) => {
            matches!(buffers.nodes.get(read.0 as usize), Some(None) | None)
                && super::super::service::known(buffers, read.0) == super::super::service::Known::Unmade
        }
        RowSource::Facts(_) | RowSource::Tree(_) | RowSource::Frontier(_) | RowSource::Observation(_) => false,
    });
    if !unmade {
        return false;
    }
    // A node the pass seldom proves empty in this worker's files is no longer asked about (learned, as the kill order is).
    let (tries, wins) = buffers.bitmap.tries[place];
    if crate::spike::bitmap_learn() && tries >= LEARN_AFTER && u32::from(wins) * LEARN_RATIO < u32::from(tries) {
        return false;
    }
    let stamp = buffers.generation.wrapping_add(1);
    let elements = run.resources.tree().kind.len();
    buffers.bitmap.begin(stamp, elements);
    let val = node_val(plan, run, node.0, buffers);
    let empty = buffers.bitmap.is_empty(val);
    let slot = &mut buffers.bitmap.tries[place];
    if slot.0 < u16::MAX {
        slot.0 += 1;
        slot.1 += u16::from(empty);
    }
    empty
}

/// The most rows of a made table the pass reads; a bigger one answers "maybe".
const TABLE_MOST: usize = 2048;

/// How many asks before a node's record decides whether it is asked again.
const LEARN_AFTER: u16 = 24;
/// A node is asked again while it is proved empty in at least one ask in this many.
const LEARN_RATIO: u32 = 8;

/// The answer of node `node`, evaluated once per file.
fn node_val(plan: &PhysicalPlan, run: &KernelRun<'_>, node: u32, buffers: &mut FileBuffers) -> u32 {
    let place = node as usize;
    let stamp = buffers.bitmap.file;
    if buffers.bitmap.stamp.len() <= place {
        let count = plan.nodes.len().max(place + 1);
        buffers.bitmap.stamp.resize(count, 0);
        buffers.bitmap.val.resize(count, 0);
    }
    if buffers.bitmap.tabled.len() <= place {
        buffers.bitmap.tabled.resize(buffers.bitmap.stamp.len(), 0);
    }
    if buffers.bitmap.tabled[place] == stamp {
        return buffers.bitmap.val[place];
    }
    // A node the file made, with no pending fault, is read from its table: its sets are exact.
    if matches!(buffers.pending.get(place), Some(None)) && matches!(buffers.nodes.get(place), Some(Some(_))) {
        let FileBuffers { bitmap, nodes, .. } = buffers;
        let Some(Some(table)) = nodes.get(place) else { return bitmap.maybe() };
        let val = if table.count.0 <= TABLE_MOST { tabled(bitmap, table) } else { bitmap.maybe() };
        bitmap.tabled[place] = stamp;
        bitmap.stamp[place] = stamp;
        bitmap.val[place] = val;
        return val;
    }
    if buffers.bitmap.stamp[place] == stamp {
        return buffers.bitmap.val[place];
    }
    if buffers.bitmap.busy.len() <= place {
        buffers.bitmap.busy.resize(buffers.bitmap.stamp.len(), 0);
    }
    if buffers.bitmap.busy[place] == stamp {
        return buffers.bitmap.maybe();
    }
    buffers.bitmap.busy[place] = stamp;
    let val = evaluated(plan, run, node, buffers);
    buffers.bitmap.busy[place] = 0;
    crate::spike::bitmap_work(1, 0);
    buffers.bitmap.stamp[place] = stamp;
    buffers.bitmap.val[place] = val;
    val
}

/// The answer of a node's input.
fn source_val(plan: &PhysicalPlan, run: &KernelRun<'_>, reader: u32, source: RowSource, buffers: &mut FileBuffers) -> u32 {
    match source {
        RowSource::Node(read) => {
            // What the file already made is known exactly.
            match super::super::service::known(buffers, read.0) {
                super::super::service::Known::Empty => return buffers.bitmap.nothing(),
                super::super::service::Known::Full | super::super::service::Known::Unmade => {}
            }
            node_val(plan, run, read.0, buffers)
        }
        RowSource::Facts(relation) => {
            // A relation's whole answer is read once per file.
            let number = ryo_relations::domain::relations::service::number_of(relation).0;
            let stamp = buffers.bitmap.file;
            if buffers.bitmap.rel_stamp.len() <= number {
                buffers.bitmap.rel_stamp.resize(number + 1, 0);
                buffers.bitmap.rel_val.resize(number + 1, 0);
            }
            if buffers.bitmap.rel_stamp[number] == stamp {
                return buffers.bitmap.rel_val[number];
            }
            let val = leaf(plan, run, reader, source, None, &[], buffers);
            buffers.bitmap.rel_stamp[number] = stamp;
            buffers.bitmap.rel_val[number] = val;
            val
        }
        RowSource::Tree(_) => leaf(plan, run, reader, source, None, &[], buffers),
        RowSource::Frontier(_) | RowSource::Observation(_) => buffers.bitmap.maybe(),
    }
}

/// The answer of a select (or none) over a relation or the tree, read in place: the bits of the rows that pass, by column.
fn leaf(
    plan: &PhysicalPlan,
    run: &KernelRun<'_>,
    node: u32,
    source: RowSource,
    select: Option<&SelectParts>,
    kept: &[ColumnIndex],
    buffers: &mut FileBuffers,
) -> u32 {
    let select = select.filter(|select| !select.comparisons.is_empty());
    if select.is_some_and(|select| !select.bindings.is_empty()) {
        return buffers.bitmap.maybe();
    }
    // A partitioned sibling reads only its bucket.
    let listed = select.is_some()
        && crate::spike::fanout()
        && match crate::domain::physical_plan::collections::kernels::partition::item::sibling_rows(
            plan,
            run,
            PlanNodeId(node),
            source,
            buffers,
            true,
        ) {
            Some(true) => return buffers.bitmap.nothing(),
            Some(false) => true,
            None => false,
        };
    let FileBuffers { bitmap, decisions, partitions, .. } = buffers;
    let list: &[u32] = if listed { &partitions.rows } else { &[] };
    let place = node as usize;
    match source {
        RowSource::Facts(relation) => {
            let Ok(view) = facts(run.inputs, relation) else {
                return bitmap.maybe();
            };
            let count = rows_of(&view).0;
            let width = view.kinds.len();
            if kept.iter().any(|column| column.0 as usize >= width) {
                return bitmap.maybe();
            }
            let out: Vec<usize> =
                if kept.is_empty() { (0..width).collect() } else { kept.iter().map(|column| column.0 as usize).collect() };
            let mut cols = Vec::with_capacity(out.len());
            for _ in &out {
                cols.push(bitmap.fresh());
            }
            let mut exact: Vec<bool> = out.iter().map(|column| matches!(view.kinds[*column], CellKind::Node | CellKind::Symbol)).collect();
            let rows = if listed { list.len() } else { count };
            crate::spike::bitmap_work(0, rows as u64);
            match select {
                None => {
                    for (slot, column) in out.iter().enumerate() {
                        let kind = view.kinds[*column];
                        let data = &view.columns[*column].0;
                        let id = cols[slot];
                        for read in 0..rows {
                            let at = if listed { list[read] as usize } else { read };
                            let Some(bits) = data.get(at) else { continue };
                            exact[slot] &= bitmap.own_place(kind, *bits);
                            let bit = bitmap.place(kind, *bits);
                            bitmap.add(id, bit);
                        }
                    }
                }
                Some(select) => {
                    let Some(decisions) = decisions.get_mut(place) else {
                        return bitmap.maybe();
                    };
                    let kernel = filter(select, view.kinds, run.resources.texts());
                    readied(decisions);
                    let mut row = std::mem::take(&mut bitmap.row);
                    let count = select.comparisons.len();
                    for read in 0..rows {
                        let at = if listed { list[read] as usize } else { read };
                        // The specialised comparisons read the columns in place; only a row they cannot decide is decoded.
                        let mut decided = Some(true);
                        if count <= FAST {
                            for fast in kernel.fast.iter().take(count) {
                                match fast_holds(fast, |column| facts_cell_at(&view, RowIndex(at), column), &kernel.texts) {
                                    Some(PredicateOutcome::Holds) => {}
                                    Some(PredicateOutcome::Fails) => {
                                        decided = Some(false);
                                        break;
                                    }
                                    None => {
                                        decided = None;
                                        break;
                                    }
                                }
                            }
                        } else {
                            decided = None;
                        }
                        match decided {
                            Some(false) => continue,
                            Some(true) => {}
                            None => {
                                decode(&view, RowIndex(at), &mut row);
                                match holds(kernel, run.matcher, &row, decisions) {
                                    Ok(PredicateOutcome::Fails) => continue,
                                    Ok(_) => {}
                                    Err(_) => {
                                        bitmap.row = row;
                                        return bitmap.maybe();
                                    }
                                }
                            }
                        }
                        for (slot, column) in out.iter().enumerate() {
                            let Some(bits) = view.columns[*column].0.get(at) else { continue };
                            exact[slot] &= bitmap.own_place(view.kinds[*column], *bits);
                            let bit = bitmap.place(view.kinds[*column], *bits);
                            let id = cols[slot];
                            bitmap.add(id, bit);
                        }
                    }
                    bitmap.row = row;
                }
            }
            bitmap.close_all(&mut cols);
            if rows == 0 {
                return bitmap.nothing();
            }
            bitmap.answer_exact(false, false, &cols, &exact)
        }
        RowSource::Tree(relation) => {
            let tree = roots(run.inputs, run.resources.tree(), PlanNodeId(node), relation);
            let count = tree_count(&tree).0;
            let width = tree.kinds.len();
            if kept.iter().any(|column| column.0 as usize >= width) {
                return bitmap.maybe();
            }
            let out: Vec<usize> =
                if kept.is_empty() { (0..width).collect() } else { kept.iter().map(|column| column.0 as usize).collect() };
            let mut cols = Vec::with_capacity(out.len());
            for _ in &out {
                cols.push(bitmap.fresh());
            }
            let rows = if listed { list.len() } else { count };
            let kernel = select.map(|select| filter(select, tree.kinds, run.resources.texts()));
            let mut exact = vec![true; out.len()];
            if let Some(decisions) = decisions.get_mut(place) {
                readied(decisions);
            }
            let mut row = std::mem::take(&mut bitmap.row);
            let mut any = false;
            for read in 0..rows {
                let at = if listed { list[read] as usize } else { read };
                tree_row(&tree, RowIndex(at), &mut row);
                if let Some(kernel) = kernel {
                    let Some(decisions) = decisions.get_mut(place) else {
                        bitmap.row = row;
                        return bitmap.maybe();
                    };
                    match holds(kernel, run.matcher, &row, decisions) {
                        Ok(PredicateOutcome::Fails) => continue,
                        Ok(_) => {}
                        Err(_) => {
                            bitmap.row = row;
                            return bitmap.maybe();
                        }
                    }
                }
                any = true;
                for (slot, column) in out.iter().enumerate() {
                    let Some(value) = row.get(*column).copied() else { continue };
                    let id = cols[slot];
                    if id == TOP {
                        continue;
                    }
                    let bits = ryo_relations::domain::relations::service::to_bits(value).0;
                    exact[slot] &= match value {
                        Value::Node(_) => bitmap.own_place(CellKind::Node, bits),
                        Value::Symbol(_) => bitmap.own_place(CellKind::Symbol, bits),
                        Value::Name(_) | Value::File(_) | Value::Span(_) | Value::Number(_) | Value::Flag(_) => false,
                    };
                    match bitmap.place_of(value) {
                        Some(bit) => bitmap.add(id, bit),
                        None => cols[slot] = TOP,
                    }
                }
            }
            bitmap.row = row;
            bitmap.close_all(&mut cols);
            if !any {
                return bitmap.nothing();
            }
            bitmap.answer_exact(false, false, &cols, &exact)
        }
        RowSource::Node(_) | RowSource::Frontier(_) | RowSource::Observation(_) => bitmap.maybe(),
    }
}

/// The ids a child or parent axis reaches from the starts' ids `from`, through the edge rows read in place; `TOP` for any
/// other direction or edge source.
fn gathered(
    run: &KernelRun<'_>,
    node: u32,
    edges: RowSource,
    axis: &crate::domain::axis::types::AxisParts,
    from: u32,
    buffers: &mut FileBuffers,
) -> u32 {
    use ryo_values::domain::plan_parts::types::Direction;
    if from == TOP {
        return TOP;
    }
    let (source, target) = match axis.direction {
        Direction::Child => (axis.parent.0 as usize, axis.child.0 as usize),
        Direction::Parent => (axis.child.0 as usize, axis.parent.0 as usize),
        _ => return TOP,
    };
    let bitmap = &mut buffers.bitmap;
    match edges {
        RowSource::Facts(relation) => {
            let Ok(view) = facts(run.inputs, relation) else { return TOP };
            let (Some(froms), Some(tos)) = (view.columns.get(source), view.columns.get(target)) else { return TOP };
            let (from_kind, to_kind) = (view.kinds[source], view.kinds[target]);
            if from_kind != CellKind::Node || to_kind != CellKind::Node {
                return TOP;
            }
            let out = bitmap.fresh();
            for (one, other) in froms.0.iter().zip(tos.0.iter()) {
                let bit = bitmap.place(from_kind, *one);
                if bitmap.contains(from, bit) {
                    let to = bitmap.place(to_kind, *other);
                    bitmap.add(out, to);
                }
            }
            let mut closed = [out];
            bitmap.close_all(&mut closed);
            closed[0]
        }
        RowSource::Tree(relation) => {
            let tree = roots(run.inputs, run.resources.tree(), PlanNodeId(node), relation);
            let count = tree_count(&tree).0;
            let out = bitmap.fresh();
            let mut row = std::mem::take(&mut bitmap.row);
            for at in 0..count {
                tree_row(&tree, RowIndex(at), &mut row);
                let (Some(one), Some(other)) = (row.get(source).copied(), row.get(target).copied()) else {
                    bitmap.row = row;
                    return TOP;
                };
                let (Some(bit), Some(to)) = (bitmap.place_of(one), bitmap.place_of(other)) else {
                    bitmap.row = row;
                    return TOP;
                };
                if bitmap.contains(from, bit) {
                    bitmap.add(out, to);
                }
            }
            bitmap.row = row;
            let mut closed = [out];
            bitmap.close_all(&mut closed);
            closed[0]
        }
        RowSource::Node(_) | RowSource::Frontier(_) | RowSource::Observation(_) => TOP,
    }
}

/// The exact answer of a made table.
fn tabled(bitmap: &mut BitmapScratch, table: &ryo_values::domain::rows::types::Table) -> u32 {
    let width = table.kinds.len();
    let count = table.count.0;
    if count == 0 {
        return bitmap.nothing();
    }
    if width == 0 {
        return bitmap.maybe();
    }
    let mut cols = Vec::with_capacity(width);
    for _ in 0..width {
        cols.push(bitmap.fresh());
    }
    let mut exact = vec![true; width];
    for (at, value) in table.cells.iter().take(count * width).enumerate() {
        let slot = at % width;
        let id = cols[slot];
        if id == TOP {
            continue;
        }
        let bits = ryo_relations::domain::relations::service::to_bits(*value).0;
        exact[slot] &= match value {
            Value::Node(_) => bitmap.own_place(CellKind::Node, bits),
            Value::Symbol(_) => bitmap.own_place(CellKind::Symbol, bits),
            Value::Name(_) | Value::File(_) | Value::Span(_) | Value::Number(_) | Value::Flag(_) => false,
        };
        match bitmap.place_of(*value) {
            Some(bit) => bitmap.add(id, bit),
            None => cols[slot] = TOP,
        }
    }
    bitmap.close_all(&mut cols);
    bitmap.answer_exact(false, false, &cols, &exact)
}

/// The input's answer with `columns` kept, in order.
fn projected(bitmap: &mut BitmapScratch, input: u32, columns: &[ColumnIndex]) -> u32 {
    if bitmap.is_empty(input) {
        return bitmap.nothing();
    }
    if bitmap.is_top(input) {
        return bitmap.maybe();
    }
    let cols: Vec<u32> = columns.iter().map(|column| bitmap.col(input, column.0 as usize)).collect();
    let exact_in = bitmap.exact_of(input);
    let exact: Vec<bool> = columns.iter().map(|column| exact_in.get(column.0 as usize).copied().unwrap_or(false)).collect();
    bitmap.answer_exact(false, false, &cols, &exact)
}

fn evaluated(plan: &PhysicalPlan, run: &KernelRun<'_>, node: u32, buffers: &mut FileBuffers) -> u32 {
    let Some(physical) = plan.nodes.get(node as usize) else {
        return buffers.bitmap.maybe();
    };
    let lowering = &physical.lowering;
    match lowering {
        Lowering::Filter { source, select } => match source {
            RowSource::Facts(_) | RowSource::Tree(_) => leaf(plan, run, node, *source, Some(select), &select.kept, buffers),
            RowSource::Node(_) | RowSource::Frontier(_) | RowSource::Observation(_) => {
                let input = source_val(plan, run, node, *source, buffers);
                let bitmap = &mut buffers.bitmap;
                if bitmap.is_empty(input) {
                    return bitmap.nothing();
                }
                if bitmap.is_top(input) {
                    return bitmap.maybe();
                }
                let mut cols = bitmap.cols_of(input);
                // Two columns compared equal hold the same values.
                if select.bindings.is_empty() {
                    for comparison in &select.comparisons {
                        if let (Test::Equal, Operand::Column(left), Operand::Column(right)) =
                            (comparison.test, &comparison.left, &comparison.right)
                        {
                            let (l, r) = (left.0 as usize, right.0 as usize);
                            if l < cols.len() && r < cols.len() {
                                let both = bitmap.and(cols[l], cols[r]);
                                cols[l] = both;
                                cols[r] = both;
                            }
                        }
                    }
                }
                let exact = if select.comparisons.is_empty() { bitmap.exact_of(input) } else { Vec::new() };
                let filtered = bitmap.answer_exact(false, false, &cols, &exact);
                if select.kept.is_empty() {
                    filtered
                } else {
                    projected(bitmap, filtered, &select.kept)
                }
            }
        },
        Lowering::Project { source, project } => {
            let input = source_val(plan, run, node, *source, buffers);
            projected(&mut buffers.bitmap, input, &project.columns)
        }
        Lowering::Extend { source, extension } => {
            let input = source_val(plan, run, node, *source, buffers);
            let bitmap = &mut buffers.bitmap;
            if bitmap.is_empty(input) {
                return bitmap.nothing();
            }
            if bitmap.is_top(input) {
                return bitmap.maybe();
            }
            let mut cols = bitmap.cols_of(input);
            cols.extend(extension.values.iter().map(|_| TOP));
            bitmap.answer(false, false, &cols)
        }
        Lowering::Probe { left, right, probe, .. } => {
            let l = source_val(plan, run, node, *left, buffers);
            if buffers.bitmap.is_empty(l) {
                return buffers.bitmap.nothing();
            }
            if matches!(probe.mode, ProbeMode::Ordered(_)) || (matches!(probe.mode, ProbeMode::Anti) && !crate::spike::bitmap_anti()) {
                // An anti probe keeps left rows it cannot decide: the left answer stands, no longer exact.
                return buffers.bitmap.inexact(l);
            }
            if matches!(probe.mode, ProbeMode::Anti) {
                // Stage 3: a one-column key whose right set is exact (and no right filter) clears the left keys it holds.
                let r = source_val(plan, run, node, *right, buffers);
                let bitmap = &mut buffers.bitmap;
                if bitmap.is_empty(r) {
                    return l;
                }
                if bitmap.is_top(r) || bitmap.is_top(l) || !probe.filter.comparisons.is_empty() {
                    return bitmap.inexact(l);
                }
                let (a, b) = match &probe.key {
                    ProbeKey::Columns { left, right } if left.len() == 1 && right.len() == 1 => (left[0].0 as usize, right[0].0 as usize),
                    ProbeKey::Whole if bitmap.cols_of(l).len() == 1 => (0, 0),
                    ProbeKey::Columns { .. } | ProbeKey::Whole => return bitmap.inexact(l),
                };
                let (lcol, rcol) = (bitmap.col(l, a), bitmap.col(r, b));
                if lcol == TOP || rcol == TOP || !bitmap.exact_of(r).get(b).copied().unwrap_or(false) {
                    return bitmap.inexact(l);
                }
                let kept = bitmap.less(lcol, rcol);
                let mut cols = bitmap.cols_of(l);
                cols[a] = kept;
                return bitmap.answer(false, false, &cols);
            }
            let r = source_val(plan, run, node, *right, buffers);
            let bitmap = &mut buffers.bitmap;
            if bitmap.is_empty(r) {
                return bitmap.nothing();
            }
            let mut lcols = bitmap.cols_of(l);
            let mut rcols = bitmap.cols_of(r);
            let pairs: Vec<(usize, usize)> = match &probe.key {
                ProbeKey::Columns { left, right } => {
                    left.iter().zip(right).map(|(a, b)| (a.0 as usize, b.0 as usize)).collect()
                }
                ProbeKey::Whole => (0..lcols.len().min(rcols.len())).map(|at| (at, at)).collect(),
            };
            let lexact = bitmap.exact_of(l);
            let rexact = bitmap.exact_of(r);
            let single = pairs.len() == 1;
            let mut key_exact: Option<(usize, usize, bool)> = None;
            for (a, b) in pairs {
                if single {
                    key_exact = Some((a, b, lexact.get(a).copied().unwrap_or(false) && rexact.get(b).copied().unwrap_or(false)));
                }
                let one = if bitmap.is_top(l) { TOP } else { lcols.get(a).copied().unwrap_or(TOP) };
                let other = if bitmap.is_top(r) { TOP } else { rcols.get(b).copied().unwrap_or(TOP) };
                let both = bitmap.and(one, other);
                if bitmap.zero(both) {
                    return bitmap.nothing();
                }
                if let Some(slot) = lcols.get_mut(a) {
                    *slot = both;
                }
                if let Some(slot) = rcols.get_mut(b) {
                    *slot = both;
                }
            }
            match probe.mode {
                ProbeMode::Semi => {
                    if bitmap.is_top(l) {
                        bitmap.maybe()
                    } else {
                        let mut exact = vec![false; lcols.len()];
                        if let Some((a, _, true)) = key_exact
                            && probe.filter.comparisons.is_empty()
                            && let Some(slot) = exact.get_mut(a)
                        {
                            *slot = true;
                        }
                        bitmap.answer_exact(false, false, &lcols, &exact)
                    }
                }
                ProbeMode::Inner => {
                    if probe.output.is_empty() {
                        return bitmap.maybe();
                    }
                    let cols: Vec<u32> = probe
                        .output
                        .iter()
                        .map(|column| {
                            let (top, cols) = match column.side {
                                Side::Left => (bitmap.is_top(l), &lcols),
                                Side::Right => (bitmap.is_top(r), &rcols),
                            };
                            if top { TOP } else { cols.get(column.column.0 as usize).copied().unwrap_or(TOP) }
                        })
                        .collect();
                    // A one-column key's values in the answer are exactly the keys both sides hold.
                    let exact: Vec<bool> = probe
                        .output
                        .iter()
                        .map(|column| match (key_exact, column.side) {
                            (Some((a, _, true)), Side::Left) => column.column.0 as usize == a && probe.filter.comparisons.is_empty(),
                            (Some((_, b, true)), Side::Right) => column.column.0 as usize == b && probe.filter.comparisons.is_empty(),
                            _ => false,
                        })
                        .collect();
                    bitmap.answer_exact(false, false, &cols, &exact)
                }
                ProbeMode::Anti | ProbeMode::Ordered(_) => bitmap.inexact(l),
            }
        }
        Lowering::Distinct { sources: [one, other] } => {
            let a = source_val(plan, run, node, *one, buffers);
            let b = source_val(plan, run, node, *other, buffers);
            let bitmap = &mut buffers.bitmap;
            match (bitmap.is_empty(a), bitmap.is_empty(b)) {
                (true, true) => bitmap.nothing(),
                (true, false) => b,
                (false, true) => a,
                (false, false) => {
                    if bitmap.is_top(a) || bitmap.is_top(b) {
                        return bitmap.maybe();
                    }
                    let (ca, cb) = (bitmap.cols_of(a), bitmap.cols_of(b));
                    if ca.len() != cb.len() {
                        return bitmap.maybe();
                    }
                    let cols: Vec<u32> = ca.iter().zip(&cb).map(|(x, y)| bitmap.or(*x, *y)).collect();
                    let (ea, eb) = (bitmap.exact_of(a), bitmap.exact_of(b));
                    let exact: Vec<bool> = ea.iter().zip(&eb).map(|(x, y)| *x && *y).collect();
                    bitmap.answer_exact(false, false, &cols, &exact)
                }
            }
        }
        Lowering::Axis { starts, edges, domain, axis, .. } => {
            let s = source_val(plan, run, node, *starts, buffers);
            if buffers.bitmap.is_empty(s) {
                return buffers.bitmap.nothing();
            }
            if let RowSource::Node(_) = domain {
                let d = source_val(plan, run, node, *domain, buffers);
                if buffers.bitmap.is_empty(d) {
                    return buffers.bitmap.nothing();
                }
            }
            let entity = if crate::spike::bitmap_axis() && axis.output.is_empty() && !buffers.bitmap.is_top(s) {
                gathered(run, node, *edges, axis, buffers.bitmap.col(s, axis.id.0 as usize), buffers)
            } else {
                TOP
            };
            let bitmap = &mut buffers.bitmap;
            if bitmap.is_top(s) {
                return bitmap.maybe();
            }
            if axis.output.is_empty() {
                let mut cols = bitmap.cols_of(s);
                cols.push(entity);
                bitmap.answer(false, false, &cols)
            } else {
                let cols: Vec<u32> = axis
                    .output
                    .iter()
                    .map(|column| match column.side {
                        Side::Left => bitmap.col(s, column.column.0 as usize),
                        Side::Right => TOP,
                    })
                    .collect();
                bitmap.answer(false, false, &cols)
            }
        }
        // Row-wise or walk kernels driven by their first input: no row in, no row out; their columns are not followed.
        Lowering::TextMap { source, .. } | Lowering::PointerChase { source, .. } => {
            let input = source_val(plan, run, node, *source, buffers);
            if buffers.bitmap.is_empty(input) { buffers.bitmap.nothing() } else { buffers.bitmap.maybe() }
        }
        Lowering::SeededReach { left: first, .. }
        | Lowering::OrderedWalk { roots: first, .. }
        | Lowering::Worklist { roots: first, .. }
        | Lowering::OrderedMatch { left: first, .. } => {
            let input = source_val(plan, run, node, *first, buffers);
            if buffers.bitmap.is_empty(input) { buffers.bitmap.nothing() } else { buffers.bitmap.maybe() }
        }
        Lowering::Lookup { .. }
        | Lowering::Group { .. }
        | Lowering::Reach { .. }
        | Lowering::Components { .. }
        | Lowering::Render { .. }
        | Lowering::ScanBound { .. }
        | Lowering::Unlowered => buffers.bitmap.maybe(),
    }
}
