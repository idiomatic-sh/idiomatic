//! Evaluate each requested witness once per file. Unknown or partial provider answers admit the file.
use super::pair_summary::{Domain, Id, Need, Program, Source};
use super::summary_schedule::Schedule;
use rustc_hash::FxHashMap as Map;
use ryo::languages::{self, FileWords, FilledColumns, NodeTable, Position, node_table};
#[path = "../../../../crates/engine/benches/vec/summary_arena.rs"]
mod arena;
#[path = "../../../../crates/engine/benches/vec/summary_classes.rs"]
mod classes;
#[path = "../../../../crates/engine/benches/vec/summary_memo.rs"]
mod memo;
#[path = "../../../../crates/engine/benches/vec/summary_points.rs"]
mod points;
#[path = "../../../../crates/engine/benches/vec/summary_postings.rs"]
pub(super) mod postings;
#[path = "../../../../crates/engine/benches/vec/summary_ready.rs"]
mod ready;
#[path = "../../../../crates/engine/benches/vec/summary_scan.rs"]
pub(super) mod scan;
#[path = "../../../../crates/engine/benches/vec/summary_witness.rs"]
mod witness;
use arena::{Arena, Bits};

#[path = "pair_counts.rs"]
mod counts;

// None is the unconstrained node set. An empty vector is the empty set.
type Nodes = Option<Bits>;
pub(super) enum Query<'a> {
    Rows,
    HasAt(Position),
    Node(languages::Column, Position),
    Exists,
    Keys(&'a mut Vec<u64>),
}
type Reader<'a> =
    dyn FnMut(Source, Query<'_>, &mut FileWords, &mut FilledColumns) -> Option<bool> + 'a;

#[derive(Default)]
struct Table {
    at: Map<u32, Option<FilledColumns>>,
    read: bool,
    answered: bool,
    complete: bool,
    cells: FilledColumns,
    index: Map<usize, Map<Id, Vec<u32>>>,
}
#[derive(Default)]
pub(super) struct Scratch {
    postings: postings::Cache,
    pub(super) literals: super::summary_literals::Observed,
    alive_words: Vec<u64>,
    pub(super) points: u64,
    pub(super) classes: classes::Classes,
    witnesses: Vec<Option<bool>>,
    pub(super) witness_calls: u64,
    matches: super::summary_values::Matches,
    facts: Vec<Table>,
    sets: memo::Memo<Nodes>,
    needs: Vec<Option<bool>>,
    arena: Arena,
    pub(super) visits: u64,
    pub(super) words: u64,
    pub(super) peak: usize,
}
struct File<'a> {
    postings: &'a mut postings::Cache,
    byte_tests: Map<(usize, &'a [u8]), bool>,
    literals: &'a mut super::summary_literals::Observed,
    p: &'a Program,
    t: &'a NodeTable,
    src: &'a [u8],
    path: Option<&'a [u8]>,
    read: &'a mut Reader<'a>,
    words: &'a mut FileWords,
    facts: &'a mut [Table],
    sets: &'a mut memo::Memo<Nodes>,
    needs: &'a mut [Option<bool>],
    arena: &'a mut Arena,
    classes: &'a mut classes::Classes,
    matches: &'a mut super::summary_values::Matches,
    order: Option<&'a Schedule>,
    visits: &'a mut u64,
    points: &'a mut u64,
    witness_calls: &'a mut u64,
    witnesses: &'a mut [Option<bool>],
    texts: Option<Map<&'a [u8], Vec<u32>>>,
    steps: u64,
}

fn insert(bits: &mut Vec<u64>, n: u32) {
    let w = (n / 64) as usize;
    if bits.len() <= w {
        bits.resize(w + 1, 0)
    }
    bits[w] |= 1 << (n % 64);
}
pub(super) fn each(b: &[u64], mut visit: impl FnMut(u32)) {
    for (i, &word) in b.iter().enumerate() {
        let mut w = word;
        while w != 0 {
            visit((i as u32) * 64 + w.trailing_zeros());
            w &= w - 1;
        }
    }
}
impl Table {
    fn load(
        &mut self,
        spec: &super::pair_summary::Fact,
        n: u32,
        words: &mut FileWords,
        read: &mut Reader<'_>,
    ) {
        if self.read {
            return;
        }
        self.read = true;
        let answered =
            read(spec.source, Query::Rows, words, &mut self.cells).expect("row query handled");
        let columns = &self.cells.0;
        self.answered = answered
            && columns.first().is_none_or(|first| {
                columns.len() == spec.columns.len()
                    && columns.iter().all(|c| c.0.len() == first.0.len())
            });
        self.complete = self.answered;
        if self.complete
            && let Some(key) = spec.key
        {
            // Providers may have node identities outside this walk. Dropping those
            // rows would make the absence of a file witness unsound.
            self.complete = self.cells.0.first().is_none_or(|c| c.0.is_empty())
                || self
                    .cells
                    .0
                    .get(key)
                    .is_some_and(|c| c.0.iter().all(|x| x.0 < u64::from(n)));
        }
    }
}

impl File<'_> {
    fn first(&self, a: Id, b: Id) -> (Id, Id) {
        if self.order.is_some_and(|s| s.ready_guards) {
            let (l, r) = (self.ready_first(a), self.ready_first(b));
            if l != r {
                return if l < r { (a, b) } else { (b, a) };
            }
        }
        if self.order.is_some_and(|o| o.cost[b] < o.cost[a]) {
            (b, a)
        } else {
            (a, b)
        }
    }
    fn complete(&mut self, slots: &[usize]) -> bool {
        let n = node_table::element_count(self.t).0 as u32;
        slots.iter().all(|&slot| {
            let table = &mut self.facts[slot];
            table.load(&self.p.facts[slot], n, self.words, self.read);
            table.complete
        })
    }
    fn pass(&mut self, id: Id) -> bool {
        if let Some(x) = self.needs[id] {
            return x;
        }
        self.steps += 1;
        let yes = match self.p.needs[id] {
            Need::Any => true,
            Need::Empty => false,
            Need::SourceContains(re) => self.p.regexes[re].is_match(self.src),
            Need::PathMatches(re) => self
                .path
                .is_none_or(|path| self.p.regexes[re].is_match(path)),
            Need::Both(..) if self.cached_guard(id, false) => false,
            Need::Both(..) if self.order.is_some() => {
                self.order.unwrap().needs[id].iter().all(|&n| self.pass(n))
            }
            Need::Either(..) if self.cached_guard(id, true) => true,
            Need::Either(..) if self.order.is_some() => {
                self.order.unwrap().needs[id].iter().any(|&n| self.pass(n))
            }
            Need::Both(a, b) => self.pass(a) && self.pass(b),
            Need::Either(a, b) => self.pass(a) || self.pass(b),
            Need::Some(x) if self.no_classes(x) => false,
            Need::Some(x) if let Some(set) = self.grouped(x, true) => {
                set.is_none_or(|s| !s.is_empty())
            }
            Need::Some(x) if self.order.is_some_and(|o| o.existential) => self.witness(x),
            Need::Some(x) => match &self.p.domains[x] {
                // All stored bitsets end at their last nonzero word. A file
                // prerequisite can stop at its first witness without allocating
                // the intersection or union of every matching node.
                _ if self.sets.get(x).is_some() => self
                    .sets
                    .get(x)
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .is_none_or(|b| !b.is_empty()),
                _ if let Some(yes) = self.local_answer(x) => yes,
                Domain::Both(a, b) if let Some(s) = self.native_join(*a, *b, true) => !s.is_empty(),
                Domain::Both(a, b) => {
                    let (a, b) = self.first(*a, *b);
                    let l = self.nodes(a);
                    if l.as_ref().is_some_and(|v| v.is_empty()) {
                        false
                    } else if let Some(set) = l.and_then(|s| self.restricted(s, b, true)) {
                        !set.is_empty()
                    } else {
                        match (l, self.nodes(b)) {
                            (Some(l), Some(r)) => self.arena.intersects(l, r),
                            (l, r) => l.or(r).is_none_or(|v| !v.is_empty()),
                        }
                    }
                }
                Domain::Either(a, b) => {
                    self.nodes(*a).is_none_or(|v| !v.is_empty())
                        || self.nodes(*b).is_none_or(|v| !v.is_empty())
                }
                Domain::Kind(k) => {
                    !node_table::ids_of_kind(self.t, languages::KindBits(*k)).is_empty()
                }
                Domain::Field(k) => {
                    !node_table::ids_of_field(self.t, languages::FieldCode(*k)).is_empty()
                }
                Domain::Fact(slot, None) => {
                    if self.order.is_some()
                        && !self.facts[*slot].read
                        && let Some(answer) = (self.read)(
                            self.p.facts[*slot].source,
                            Query::Exists,
                            self.words,
                            &mut FilledColumns::default(),
                        )
                    {
                        self.needs[id] = Some(answer);
                        return answer;
                    }
                    let table = &mut self.facts[*slot];
                    table.load(
                        &self.p.facts[*slot],
                        node_table::element_count(self.t).0 as u32,
                        self.words,
                        self.read,
                    );
                    !table.complete || table.cells.0.first().is_some_and(|c| !c.0.is_empty())
                }
                _ => self.nodes(x).is_none_or(|b| !b.is_empty()),
            },
        };
        self.needs[id] = Some(yes);
        yes
    }
    fn no_classes(&mut self, id: Id) -> bool {
        self.order.is_some_and(|o| o.classes)
            && matches!(
                self.p.domains[id],
                Domain::Both(..)
                    | Domain::Parents(_)
                    | Domain::Children(_)
                    | Domain::When(..)
                    | Domain::ProvenExcept(..)
                    | Domain::CountUnder(..)
            )
            && self.classes.empty(self.p, self.t, self.texts.as_ref(), id)
    }
    fn nodes(&mut self, id: Id) -> Nodes {
        if let Some(s) = self.sets.get(id) {
            return s;
        }
        if self.order.is_some_and(|o| o.ready_guards)
            && self.witnesses.get(id) == Some(&Some(false))
        {
            let empty = Some(Bits::default());
            self.sets.set(id, empty);
            return empty;
        }
        if let Some(set) = self.grouped(id, false) {
            self.sets.set(id, set);
            return set;
        }
        if let Some(set) = self.local_set(id) {
            self.sets.set(id, set);
            return set;
        }
        if self.no_classes(id) {
            let empty = Some(Bits::default());
            self.sets.set(id, empty);
            return empty;
        }
        if let Some(child) = self.order.and_then(|o| o.alias.get(id)).copied().flatten() {
            let s = self.nodes(child);
            self.sets.set(id, s);
            return s;
        }
        self.steps += 1;
        let n =
            u32::try_from(node_table::element_count(self.t).0).expect("table positions fit u32");
        let nodes = match &self.p.domains[id] {
            Domain::Any => None,
            Domain::CountUnder(rows, depth, op, value) => {
                let (rows, depth, op, value) = (*rows, *depth, *op, *value);
                let children = self.nodes(rows);
                counts::under(
                    self.t,
                    self.arena,
                    children,
                    matches!(self.p.domains[rows], Domain::Any),
                    depth,
                    op,
                    value,
                )
            }
            Domain::HasParent => Some(
                self.arena
                    .bits((1..n).filter(|&i| node_table::parent_of(self.t, Position(i)).0 != i)),
            ),
            Domain::Empty => Some(Bits::default()),
            Domain::Kind(k) => Some(
                self.arena.bits(
                    node_table::ids_of_kind(self.t, languages::KindBits(*k))
                        .iter()
                        .map(|p| p.0),
                ),
            ),
            Domain::Field(k) => Some(
                self.arena.bits(
                    node_table::ids_of_field(self.t, languages::FieldCode(*k))
                        .iter()
                        .map(|p| p.0),
                ),
            ),
            Domain::SpanMatching(k, re) => {
                let cache = self.order.is_some_and(|o| o.byte_regex);
                let regex = &self.p.regexes[*re];
                Some(
                    self.arena.bits(
                        node_table::ids_of_kind(self.t, languages::KindBits(*k))
                            .iter()
                            .filter_map(|pos| {
                                let text = &self.src[super::span_bytes(self.t, pos.0 as usize)];
                                let yes = if cache {
                                    *self
                                        .byte_tests
                                        .entry((*re, text))
                                        .or_insert_with(|| regex.is_match(text))
                                } else {
                                    regex.is_match(text)
                                };
                                yes.then_some(pos.0)
                            }),
                    ),
                )
            }
            Domain::TextMatching(k, re) => {
                let regex = &self.p.regexes[*re];
                let cache = self.order.is_some_and(|o| o.byte_regex);
                Some(
                    self.arena.bits(
                        node_table::ids_of_kind(self.t, languages::KindBits(*k))
                            .iter()
                            .filter_map(|p| {
                                let text = node_table::decoded_of(self.t, *p).map_or_else(
                                    || &self.src[super::span_bytes(self.t, p.0 as usize)],
                                    |t| t.0.as_bytes(),
                                );
                                (if cache {
                                    *self
                                        .byte_tests
                                        .entry((*re, text))
                                        .or_insert_with(|| regex.is_match(text))
                                } else {
                                    regex.is_match(text)
                                })
                                .then_some(p.0)
                            }),
                    ),
                )
            }
            Domain::Text(_) if let Some(layout) = self.order.and_then(|o| o.literals.as_ref()) => {
                Some(
                    self.arena.bits(
                        layout
                            .nodes(id, self.t, self.src, self.literals)
                            .iter()
                            .copied(),
                    ),
                )
            }
            Domain::Text(t) => {
                let by_text = self.texts.get_or_insert_with(|| {
                    let mut m = Map::<&[u8], Vec<u32>>::default();
                    for i in self.p.text_kinds.iter().flat_map(|k| {
                        node_table::ids_of_kind(self.t, languages::KindBits(*k))
                            .iter()
                            .map(|p| p.0)
                    }) {
                        let value = node_table::decoded_of(self.t, Position(i)).map_or_else(
                            || &self.src[super::span_bytes(self.t, i as usize)],
                            |t| t.0.as_bytes(),
                        );
                        if value.len() <= self.p.max_text && self.p.texts.contains(value) {
                            m.entry(value).or_default().push(i);
                        }
                    }
                    m
                });
                Some(
                    self.arena
                        .bits(by_text.get(t.as_bytes()).into_iter().flatten().copied()),
                )
            }
            Domain::Both(a, b) if let Some(s) = self.native_join(*a, *b, false) => Some(s),
            Domain::Both(a, b) => {
                let (a, b) = self.first(*a, *b);
                let left = self.nodes(a);
                if left.is_some_and(Bits::is_empty) {
                    left
                } else if let Some(set) = left.and_then(|s| self.restricted(s, b, false)) {
                    Some(set)
                } else {
                    match (left, self.nodes(b)) {
                        (Some(a), Some(b)) => Some(self.arena.intersect(a, b)),
                        (a, b) => a.or(b),
                    }
                }
            }
            Domain::When(need, nodes) => {
                if self.pass(*need) {
                    self.nodes(*nodes)
                } else {
                    Some(Bits::default())
                }
            }
            // The excluded side must be exact in this particular file.
            Domain::ProvenExcept(left, difference, guard, slots)
                if self.order.is_some_and(|o| o.proof_order) =>
            {
                let a = self.nodes(*left);
                if a.is_some_and(Bits::is_empty) || self.pass(*guard) {
                    a
                } else {
                    let d = self.nodes(*difference);
                    if self.arena.identical(a, d) || self.complete(slots) {
                        d
                    } else {
                        a
                    }
                }
            }
            Domain::ProvenExcept(left, difference, guard, slots) => {
                let admitted = self.nodes(*left);
                if admitted.as_ref().is_some_and(|v| v.is_empty())
                    || self.pass(*guard)
                    || !self.complete(slots)
                {
                    admitted
                } else {
                    self.nodes(*difference)
                }
            }
            Domain::Except(a, b) => {
                let left = self.nodes(*a);
                if left.as_ref().is_some_and(|v| v.is_empty()) {
                    self.sets.set(id, left.clone());
                    return left;
                }
                let Some(right) = self.nodes(*b) else {
                    // A missing provider answer cannot prove an exclusion.
                    let out = if matches!(self.p.domains[*b], Domain::Any) {
                        Some(Bits::default())
                    } else {
                        left
                    };
                    self.sets.set(id, out.clone());
                    return out;
                };
                let mut out = left.map_or_else(
                    || {
                        let mut all = vec![u64::MAX; (n as usize).div_ceil(64)];
                        if n % 64 != 0 {
                            *all.last_mut().unwrap() = (1u64 << (n % 64)) - 1;
                        }
                        all
                    },
                    |v| self.arena.absolute(v),
                );
                for (w, r) in out
                    .iter_mut()
                    .skip(right.first())
                    .zip(&self.arena.words[right.range()])
                {
                    *w &= !*r
                }
                while out.last() == Some(&0) {
                    out.pop();
                }
                Some(self.arena.commit(&out))
            }
            Domain::Either(a, b) => match (self.nodes(*a), self.nodes(*b)) {
                (Some(a), Some(b)) => Some(self.arena.union(a, b)),
                _ => None,
            },
            Domain::Descendants(root) => self.nodes(*root).map(|ancestors| {
                if node_table::descendant_ranges(self.t, Position(0)).is_some() {
                    let mut seen = Vec::new();
                    let mut through = 0u32;
                    each(&self.arena.words[ancestors.range()], |offset| {
                        let n = ancestors.first() as u32 * 64 + offset;
                        if n < through {
                            return;
                        }
                        let ranges = node_table::descendant_ranges(self.t, Position(n))
                            .expect("recorded intervals");
                        through = ranges.nodes.end;
                        arena::cover(&mut seen, ranges.nodes);
                        arena::cover(&mut seen, ranges.tokens);
                    });
                    return self.arena.commit(&seen);
                }
                let mut pending = Vec::new();
                each(&self.arena.words[ancestors.range()], |n| {
                    pending.push(ancestors.first() as u32 * 64 + n)
                });
                let mut seen = Vec::new();
                while let Some(parent) = pending.pop() {
                    for child in node_table::children_of(self.t, Position(parent)) {
                        let pos = child.0;
                        if !seen
                            .get((pos / 64) as usize)
                            .is_some_and(|w| (*w >> (pos % 64)) & 1 != 0)
                        {
                            insert(&mut seen, pos);
                            pending.push(pos);
                        }
                    }
                }
                self.arena.commit(&seen)
            }),
            Domain::Parents(x) | Domain::Children(x) | Domain::Ancestors(x) => {
                let children = matches!(self.p.domains[id], Domain::Children(_));
                let ancestors = matches!(self.p.domains[id], Domain::Ancestors(_));
                let input = if matches!(self.p.domains[*x], Domain::Any) {
                    Some(self.arena.bits(0..n))
                } else {
                    self.nodes(*x)
                };
                input.map(|b| {
                    let out = &mut self.arena.work;
                    each(&self.arena.words[b.range()], |offset| {
                        let i = b.first() as u32 * 64 + offset;
                        if children {
                            if let Some(children) = node_table::direct_children(self.t, Position(i))
                            {
                                for child in children {
                                    insert(out, child.0);
                                }
                            } else {
                                for c in node_table::children_of(self.t, Position(i)) {
                                    insert(out, c.0);
                                }
                            }
                        } else {
                            let mut child = i;
                            loop {
                                let p = node_table::parent_of(self.t, Position(child)).0;
                                if p == child || p >= n {
                                    break;
                                }
                                if ancestors
                                    && out
                                        .get((p / 64) as usize)
                                        .is_some_and(|w| (*w >> (p % 64)) & 1 != 0)
                                {
                                    break;
                                }
                                insert(out, p);
                                if !ancestors {
                                    break;
                                }
                                child = p;
                            }
                        }
                    });
                    self.arena.finish()
                })
            }
            Domain::RelatedIds(slot, endpoint, filter) => {
                let table = &mut self.facts[*slot];
                table.load(&self.p.facts[*slot], n, self.words, self.read);
                if !table.answered {
                    None
                } else if let Some(keys) = table.cells.0.get(*endpoint) {
                    let values = filter
                        .as_ref()
                        .and_then(|(col, ids)| Some((table.cells.0.get(*col)?, ids)));
                    if filter.is_some() && values.is_none_or(|(c, _)| c.0.len() != keys.0.len()) {
                        None
                    } else {
                        let mut valid = true;
                        let nodes =
                            self.arena
                                .bits(keys.0.iter().enumerate().filter_map(|(i, k)| {
                                    let keep = values.is_none_or(|(c, ids)| {
                                        super::summary_values::symbol(
                                            self.order, self.p, self.words, c.0[i],
                                        )
                                        .as_ref()
                                        .is_some_and(|id| ids.binary_search(id).is_ok())
                                    });
                                    if keep && k.0 >= u64::from(n) {
                                        valid = false;
                                        return None;
                                    }
                                    keep.then_some(k.0 as u32)
                                }));
                        valid.then_some(nodes)
                    }
                } else {
                    None
                }
            }
            Domain::Flag(slot, col, _) | Domain::Number(slot, col, _, _) => {
                let spec = &self.p.facts[*slot];
                let table = &mut self.facts[*slot];
                table.load(spec, n, self.words, self.read);
                if table.complete {
                    let key = spec.key.and_then(|k| table.cells.0.get(k));
                    Some(
                        self.arena.bits(
                            table
                                .cells
                                .0
                                .get(*col)
                                .into_iter()
                                .flat_map(|c| &c.0)
                                .enumerate()
                                .filter(|(_, cell)| match self.p.domains[id] {
                                    Domain::Flag(_, _, v) => cell.0 == u64::from(v),
                                    Domain::Number(_, _, op, v) => {
                                        counts::compare(cell.0 as i64, op, v)
                                    }
                                    _ => false,
                                })
                                .map(|(i, _)| key.map_or(0, |ids| ids.0[i].0 as u32)),
                        ),
                    )
                } else {
                    None
                }
            }
            Domain::SameRow(slot, tests) => {
                use super::pair_summary::CellTest;
                let spec = &self.p.facts[*slot];
                let table = &mut self.facts[*slot];
                table.load(spec, n, self.words, self.read);
                if !table.complete {
                    None
                } else {
                    let key = spec.key.and_then(|k| table.cells.0.get(k));
                    let len = table.cells.0.first().map_or(0, |c| c.0.len());
                    Some(self.arena.bits((0..len).filter_map(|row| {
                        let matched = tests.iter().all(|test| match test {
                            CellTest::Symbols(c, values) => {
                                let cell = table.cells.0[*c].0[row];
                                cell.0 != super::finish::NULL
                                    && super::summary_values::symbol(
                                        self.order, self.p, self.words, cell,
                                    )
                                    .as_ref()
                                    .is_some_and(|id| values.binary_search(id).is_ok())
                            }
                            CellTest::Numeric(c, op, literal) => {
                                counts::compare(table.cells.0[*c].0[row].0 as i64, *op, *literal)
                            }
                        });
                        matched.then(|| key.map_or(0, |keys| keys.0[row].0 as u32))
                    })))
                }
            }
            Domain::OtherValues(slot, col, excluded) => {
                let spec = &self.p.facts[*slot];
                let table = &mut self.facts[*slot];
                table.load(spec, n, self.words, self.read);
                if !table.complete {
                    None
                } else {
                    let key = spec.key.and_then(|k| table.cells.0.get(k));
                    Some(
                        self.arena.bits(
                            table
                                .cells
                                .0
                                .get(*col)
                                .into_iter()
                                .flat_map(|c| c.0.iter())
                                .enumerate()
                                .filter_map(|(i, cell)| {
                                    super::summary_values::symbol(
                                        self.order, self.p, self.words, *cell,
                                    )
                                    .as_ref()
                                    .is_none_or(|id| excluded.binary_search(id).is_err())
                                    .then(|| key.map_or(0, |ids| ids.0[i].0 as u32))
                                }),
                        ),
                    )
                }
            }
            Domain::CapturedSuffix(slot, col, delimiter, re) => {
                let spec = &self.p.facts[*slot];
                let table = &mut self.facts[*slot];
                table.load(spec, n, self.words, self.read);
                if !table.complete {
                    None
                } else {
                    let key = spec.key.and_then(|k| table.cells.0.get(k));
                    let regex = &self.p.regexes[*re];
                    Some(
                        self.arena.bits(
                            table
                                .cells
                                .0
                                .get(*col)
                                .into_iter()
                                .flat_map(|c| c.0.iter())
                                .enumerate()
                                .filter_map(|(i, cell)| {
                                    let text =
                                        languages::facts::text(self.words, *cell).0.as_bytes();
                                    text.windows(delimiter.len())
                                        .enumerate()
                                        .any(|(p, w)| {
                                            w == delimiter
                                                && regex.is_match(&text[p + delimiter.len()..])
                                        })
                                        .then(|| key.map_or(0, |ids| ids.0[i].0 as u32))
                                }),
                        ),
                    )
                }
            }
            Domain::Matching(slot, col, re) => {
                let spec = &self.p.facts[*slot];
                let table = &mut self.facts[*slot];
                table.load(spec, n, self.words, self.read);
                if !table.complete {
                    None
                } else {
                    let key = spec.key.and_then(|k| table.cells.0.get(k));
                    let regex = &self.p.regexes[*re];
                    Some(
                        self.arena.bits(
                            table
                                .cells
                                .0
                                .get(*col)
                                .into_iter()
                                .flat_map(|c| c.0.iter())
                                .enumerate()
                                .filter_map(|(r, cell)| {
                                    let yes = if spec.kinds[*col] == languages::CellKind::Span {
                                        let span =
                                            (cell.0 >> 32) as usize..(cell.0 as u32) as usize;
                                        self.src.get(span).is_some_and(|text| regex.is_match(text))
                                    } else if self.order.is_some_and(|o| o.cache_regex) {
                                        self.matches.test(self.p, *re, *cell, self.words)
                                    } else {
                                        regex.is_match(
                                            languages::facts::text(self.words, *cell).0.as_bytes(),
                                        )
                                    };
                                    yes.then(|| key.map_or(0, |c| c.0[r].0 as u32))
                                        .filter(|x| spec.key.is_none() || *x < n)
                                }),
                        ),
                    )
                }
            }
            Domain::Fact(_, Some(_)) if let Some(set) = self.posted(id, false) => set,
            Domain::Fact(slot, test) => {
                let spec = &self.p.facts[*slot];
                if self.order.is_some()
                    && test.is_none()
                    && !self.facts[*slot].read
                    && spec.key.is_some()
                {
                    let mut result = Vec::new();
                    if let Some(complete) = (self.read)(
                        spec.source,
                        Query::Keys(&mut result),
                        self.words,
                        &mut FilledColumns::default(),
                    ) {
                        let value = complete.then(|| self.arena.commit(&result));
                        self.sets.set(id, value);
                        return value;
                    }
                }
                let table = &mut self.facts[*slot];
                table.load(spec, n, self.words, self.read);
                if !table.complete {
                    None
                } else {
                    let key = spec.key.and_then(|k| table.cells.0.get(k));
                    let node = |r: usize| key.map_or(0, |c| c.0[r].0 as u32);
                    if let Some((col, values)) = test {
                        let idx = table.index.entry(*col).or_insert_with(|| {
                            let mut m = Map::<Id, Vec<u32>>::default();
                            if let Some(column) = table.cells.0.get(*col) {
                                for (r, &cell) in column.0.iter().enumerate() {
                                    if let Some(id) = super::summary_values::symbol(
                                        self.order, self.p, self.words, cell,
                                    ) {
                                        m.entry(id).or_default().push(node(r));
                                    }
                                }
                            }
                            m
                        });
                        Some(
                            self.arena
                                .bits(values.iter().filter_map(|v| idx.get(v)).flatten().copied()),
                        )
                    } else {
                        Some(
                            self.arena.bits(
                                (0..table.cells.0.first().map_or(0, |c| c.0.len()))
                                    .map(node)
                                    .filter(|x| spec.key.is_none() || *x < n),
                            ),
                        )
                    }
                }
            }
        };
        self.sets.set(id, nodes.clone());
        nodes
    }
}

impl Program {
    pub(super) fn file(
        &self,
        t: &NodeTable,
        src: &[u8],
        path: Option<&[u8]>,
        words: &mut FileWords,
        mut read: impl FnMut(Source, &mut FileWords, &mut FilledColumns) -> bool,
        alive: &mut [bool],
        exact: Option<&mut Vec<(usize, Vec<u64>)>>,
    ) -> u64 {
        self.eval(
            &mut Scratch::default(),
            None,
            t,
            src,
            path,
            words,
            |source, ask, words, out| match ask {
                Query::Rows => Some(read(source, words, out)),
                _ => None,
            },
            alive,
            exact,
        )
    }
    pub(super) fn eval(
        &self,
        scratch: &mut Scratch,
        order: Option<&Schedule>,
        t: &NodeTable,
        src: &[u8],
        path: Option<&[u8]>,
        words: &mut FileWords,
        mut read: impl FnMut(Source, Query<'_>, &mut FileWords, &mut FilledColumns) -> Option<bool>,
        alive: &mut [bool],
        mut exact: Option<&mut Vec<(usize, Vec<u64>)>>,
    ) -> u64 {
        languages::facts::clear_words(words);
        scratch.arena.words.clear();
        scratch.sets.reset(self.domains.len());
        scratch.arena.direct = order.is_some_and(|s| s.direct_masks);
        scratch.arena.window = order.is_some_and(|s| s.windows);
        scratch.needs.clear();
        scratch.needs.resize(self.needs.len(), None);
        scratch.facts.resize_with(self.facts.len(), Table::default);
        for t in &mut scratch.facts {
            t.read = false;
            t.answered = false;
            t.complete = false;
            t.index.clear();
            t.at.clear();
            for c in &mut t.cells.0 {
                c.0.clear();
            }
        }
        if order.is_some_and(|o| o.cache_regex) {
            scratch.matches.reset(
                self.regexes.len(),
                languages::facts::base_count(words).0 as usize,
            );
        }
        scratch.witnesses.clear();
        if order.is_some_and(|o| o.existential) {
            scratch.witnesses.resize(self.domains.len(), None);
        }
        scratch.classes.reset();
        scratch
            .postings
            .reset(order.and_then(|s| s.postings.as_ref()));
        let mut f = File {
            postings: &mut scratch.postings,
            literals: &mut scratch.literals,
            byte_tests: Map::default(),
            classes: &mut scratch.classes,
            witnesses: &mut scratch.witnesses,
            witness_calls: &mut scratch.witness_calls,
            matches: &mut scratch.matches,
            p: self,
            t,
            src,
            path,
            read: &mut read,
            words,
            facts: &mut scratch.facts,
            arena: &mut scratch.arena,
            order,
            visits: &mut scratch.visits,
            points: &mut scratch.points,
            sets: &mut scratch.sets,
            needs: &mut scratch.needs,
            texts: None,
            steps: 0,
        };
        if let Some(rules) = order.and_then(|s| s.rules.as_ref()) {
            rules.apply(alive, &mut scratch.alive_words, |id| f.pass(id));
        } else {
            for (ri, (a, (_, root))) in alive.iter_mut().zip(&self.roots).enumerate() {
                if *a {
                    *a = if let Some(branches) = order.and_then(|s| s.branches.as_ref()) {
                        let mut pc = branches.entry[ri];
                        while pc >= 2 {
                            let b = branches.code[pc - 2];
                            pc = if f.pass(b.test) { b.yes } else { b.no };
                        }
                        pc == 1
                    } else {
                        f.pass(*root)
                    }
                }
            }
        }
        if let Some(out) = exact.as_mut() {
            for (rule, cert) in &self.exact {
                let rule = *rule;
                if alive[rule] && !f.pass(cert.guard) && f.complete(&cert.facts) {
                    let nodes = f
                        .nodes(cert.nodes)
                        .expect("certified domains contain only total node tests");
                    out.push((rule, f.arena.absolute(nodes)));
                }
            }
        }
        scratch.words += f.arena.words.len() as u64;
        scratch.peak = scratch.peak.max(f.arena.words.len());
        f.steps
    }
}

impl Scratch {
    pub(super) fn report_postings(&self) {
        println!(
            "shared symbol postings columns {} rows visited {} retained {}",
            self.postings.columns, self.postings.rows, self.postings.kept
        );
    }
    pub(super) fn measure_masks(&mut self) {
        self.arena.census = Some(Default::default());
    }
    pub(super) fn report_masks(&self) {
        if let Some(c) = &self.arena.census {
            c.report();
        }
    }
}
