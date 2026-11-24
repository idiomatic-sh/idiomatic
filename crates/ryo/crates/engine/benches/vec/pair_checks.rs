//! Evaluate each requested witness once per file. Unknown or partial provider answers admit the file.
use super::pair_summary::cost::{self, Sample};
use super::pair_summary::{Domain, Id, Need, Program, Source};
use super::summary_schedule::Schedule;
use rustc_hash::FxHashMap as Map;
use ryo::languages::{self, FileWords, FilledColumns, NodeTable, Position, node_table};
#[path = "summary_arena.rs"]
mod arena;
#[path = "summary_memo.rs"]
mod memo;
#[path = "summary_postings.rs"]
pub(super) mod postings;
#[path = "summary_ready.rs"]
mod ready;
#[path = "summary_scan.rs"]
pub(super) mod scan;
#[path = "summary_witness.rs"]
mod physical_witness;
use arena::{Arena, Bits};

#[path = "pair_counts.rs"]
mod counts;
#[path = "pair_witness.rs"]
mod witness;

// None is the unconstrained node set. An empty vector is the empty set.
type Nodes = Option<Bits>;
/// The exact syntax text column supplied by a host with per-position symbol IDs.
#[derive(Clone, Copy)]
pub(super) struct SyntaxText<'a> {
    pub ids: &'a [u32],
    pub symbols: &'a [std::rc::Rc<[u8]>],
}
impl<'a> SyntaxText<'a> {
    fn at(self, index: usize) -> &'a [u8] {
        &self.symbols[self.ids[index] as usize]
    }
}
pub(super) enum Query<'a> {
    Rows,
    Exists,
    Keys(&'a mut Vec<u64>),
}
type Reader<'a> =
    dyn FnMut(Source, Query<'_>, &mut FileWords, &mut FilledColumns) -> Option<bool> + 'a;

#[derive(Default)]
struct Table {
    read: bool,
    answered: bool,
    complete: bool,
    cells: FilledColumns,
}
#[derive(Default)]
pub(super) struct Scratch {
    postings: postings::Cache,
    pub(super) literals: super::summary_literals::Observed,
    alive_words: Vec<u64>,
    witnesses: Vec<Option<bool>>,
    matches: super::summary_values::Matches,
    facts: Vec<Table>,
    sets: memo::Memo<Nodes>,
    needs: Vec<Option<bool>>,
    arena: Arena,
    #[cfg(feature = "summary-diagnostics")]
    pub(super) words: u64,
    #[cfg(feature = "summary-diagnostics")]
    pub(super) peak: usize,
}
struct File<'a, const PROFILE: bool> {
    owner: usize,
    reader_class: &'a std::cell::Cell<usize>,
    file_owner: Vec<usize>,
    node_owner: Vec<usize>,
    plain_tree: bool,
    stats: [Sample; 13],
    charged: f64,
    overhead: f64,
    postings: &'a mut postings::Cache,
    byte_tests: Map<(usize, &'a [u8]), bool>,
    literals: &'a mut super::summary_literals::Observed,
    p: &'a Program,
    t: &'a NodeTable,
    src: &'a [u8],
    syntax_text: Option<SyntaxText<'a>>,
    path: Option<&'a [u8]>,
    read: &'a mut Reader<'a>,
    words: &'a mut FileWords,
    facts: &'a mut [Table],
    sets: &'a mut memo::Memo<Nodes>,
    needs: &'a mut [Option<bool>],
    arena: &'a mut Arena,
    matches: &'a mut super::summary_values::Matches,
    order: &'a Schedule,
    witnesses: &'a mut [Option<bool>],
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

impl<const PROFILE: bool> File<'_, PROFILE> {
    fn first(&self, a: Id, b: Id) -> (Id, Id) {
        let (l, r) = (self.ready_first(a), self.ready_first(b));
        if l != r {
            return if l < r { (a, b) } else { (b, a) };
        }
        if self.order.cost[b] < self.order.cost[a] {
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
        let class = if PROFILE {
            cost::file_family(self.p, id) as usize
        } else {
            0
        };
        if PROFILE {
            self.stats[class].calls += 1;
        }
        if let Some(result) = &self.needs[id] {
            if PROFILE {
                self.stats[class].hits += 1;
                self.stats[class].cross += u64::from(self.file_owner[id] != self.owner);
            }
            return *result;
        }
        if !PROFILE {
            return self.pass_miss(id);
        }
        self.file_owner[id] = self.owner;
        let prior = self.charged;
        let start = super::thread_cpu();
        let caller = self.reader_class.replace(class);
        let out = self.pass_miss(id);
        self.reader_class.set(caller);
        let elapsed = super::thread_cpu() - start;
        self.stats[class].ns += (elapsed - (self.charged - prior) - self.overhead) * 1e9;
        self.charged = prior + elapsed + self.overhead;
        out
    }
    fn pass_miss(&mut self, id: Id) -> bool {
        self.steps += 1;
        let yes = match self.p.needs[id] {
            Need::InvalidPattern(_) => unreachable!("proof validation happens at load"),
            Need::Any => true,
            Need::Empty => false,
            Need::SourceContains(re) => self.p.regexes[re].is_match(self.src),
            Need::PathGlob(slot) => self.path.is_none_or(|path| {
                use ryo_matching::inbound::use_cases::matching::{
                    interface::MatchingUseCase, types::{GlobPattern, MatchOutcome, SearchText},
                    use_case::MatchingUseCaseImpl,
                };
                MatchingUseCaseImpl::new().matches_text_glob(
                    GlobPattern(&self.p.path_globs[slot]), SearchText(&String::from_utf8_lossy(path)),
                ) == MatchOutcome::Matches
            }),
            Need::PathMatches(re) => self
                .path
                .is_none_or(|path| self.p.regexes[re].is_match(path)),
            Need::Terms(all, ref terms) => {
                if all {
                    terms.iter().all(|i| self.pass(*i))
                } else {
                    terms.iter().any(|i| self.pass(*i))
                }
            }
            Need::Both(..) if self.cached_guard(id, false) => false,
            Need::Both(..) => self.order.needs[id].iter().all(|&n| self.pass(n)),
            Need::Either(..) if self.cached_guard(id, true) => true,
            Need::Either(..) => self.order.needs[id].iter().any(|&n| self.pass(n)),
            Need::Some(x) => self.witness(x),
        };
        self.needs[id] = Some(yes);
        yes
    }
    fn nodes(&mut self, id: Id) -> Nodes {
        let class = if PROFILE {
            cost::family(&self.p.domains[id], &self.p.facts) as usize
        } else {
            0
        };
        if PROFILE {
            self.stats[class].calls += 1;
        }
        if let Some(result) = self.sets.get(id) {
            if PROFILE {
                self.stats[class].hits += 1;
                self.stats[class].cross += u64::from(self.node_owner[id] != self.owner);
            }
            return result.clone();
        }
        if !PROFILE {
            return self.nodes_miss(id);
        }
        self.node_owner[id] = self.owner;
        let prior = self.charged;
        let start = super::thread_cpu();
        let caller = self.reader_class.replace(class);
        let out = self.nodes_miss(id);
        self.reader_class.set(caller);
        let elapsed = super::thread_cpu() - start;
        self.stats[class].ns += (elapsed - (self.charged - prior) - self.overhead) * 1e9;
        self.charged = prior + elapsed + self.overhead;
        out
    }
    fn nodes_miss(&mut self, id: Id) -> Nodes {
        if self.witnesses.get(id) == Some(&Some(false)) {
            let empty = Some(Bits::default());
            self.sets.set(id, empty);
            return empty;
        }
        if let Some(child) = self.order.alias[id] {
            let s = self.nodes(child);
            self.sets.set(id, s);
            return s;
        }
        self.steps += 1;
        let n =
            u32::try_from(node_table::element_count(self.t).0).expect("table positions fit u32");
        if self.plain_tree
            && let Some(edge) = self.p.paths[id]
        {
            let mut output = Vec::new();
            if self.edge(edge, Some(&mut output)).is_some() {
                let nodes = Some(self.arena.commit(&output));
                self.sets.set(id, nodes);
                return nodes;
            }
        }
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
            Domain::SpanMatching(k, re) | Domain::TextMatching(k, re) => {
                let decoded = matches!(self.p.domains[id], Domain::TextMatching(..));
                let regex = &self.p.regexes[*re];
                Some(
                    self.arena.bits(
                        node_table::ids_of_kind(self.t, languages::KindBits(*k))
                            .iter()
                            .filter_map(|&pos| {
                                let raw = || &self.src[super::span_bytes(self.t, pos.0 as usize)];
                                let text = if decoded && let Some(column) = self.syntax_text {
                                    column.at(pos.0 as usize)
                                } else if decoded {
                                    node_table::decoded_of(self.t, pos)
                                        .map_or_else(raw, |s| s.0.as_bytes())
                                } else {
                                    raw()
                                };
                                self.byte_tests
                                    .entry((*re, text))
                                    .or_insert_with(|| regex.is_match(text))
                                    .then_some(pos.0)
                            }),
                    ),
                )
            }
            Domain::Text(_) => Some(
                self.arena.bits(
                    self.order
                        .literals
                        .nodes(id, self.t, self.src, self.literals)
                        .iter()
                        .copied(),
                ),
            ),
            Domain::Both(a, b) => {
                let (a, b) = self.first(*a, *b);
                let left = self.nodes(a);
                if left.is_some_and(Bits::is_empty) {
                    left
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
            Domain::ProvenExcept(left, difference, guard, slots) => {
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
            // An unconstrained arm admits every node. Only a known left mask
            // can justify requesting the right arm and its provider tables.
            Domain::Either(a, b) => self.nodes(*a).and_then(|left| {
                let right = self.nodes(*b)?;
                Some(self.arena.union(left, right))
            }),
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
                                            self.order, self.words, c.0[i],
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
                                    && super::summary_values::symbol(self.order, self.words, cell)
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
                                    super::summary_values::symbol(self.order, self.words, *cell)
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
            Domain::Member(slot, col, project, field) => {
                let (slot, col, project, field) = (*slot, *col, *project, *field);
                let available = &mut self.facts[project];
                available.load(&self.p.facts[project], n, self.words, self.read);
                if !available.complete {
                    None
                } else if available.cells.0.first().is_none_or(|c| c.0.is_empty()) {
                    Some(Bits::default())
                } else {
                    let allowed: Vec<_> = available.cells.0[field].0.iter().map(|v| v.0).collect();
                    let t = &mut self.facts[slot];
                    let spec = &self.p.facts[slot];
                    t.load(spec, n, self.words, self.read);
                    if !t.complete {
                        None
                    } else {
                        let key = spec.key.and_then(|k| t.cells.0.get(k));
                        Some(self.arena.bits(
                            t.cells
                                .0
                                .get(col)
                                .into_iter()
                                .flat_map(|c| &c.0)
                                .enumerate()
                                .filter(|(_, c)| allowed.contains(&c.0))
                                .map(|(i, _)| key.map_or(0, |k| k.0[i].0 as u32)),
                        ))
                    }
                }
            }
            Domain::Matching(slot, col, re) | Domain::Unmatched(slot, col, re) => {
                let negative = matches!(self.p.domains[id], Domain::Unmatched(..));
                let spec = &self.p.facts[*slot];
                let table = &mut self.facts[*slot];
                table.load(spec, n, self.words, self.read);
                if !table.complete {
                    None
                } else {
                    let key = spec.key.and_then(|k| table.cells.0.get(k));
                    let Some(regex) = self.p.regexes[*re].compiled() else {
                        self.sets.set(id, None);
                        return None;
                    };
                    Some(self.arena.bits(
                        table
                            .cells
                            .0
                            .get(*col)
                            .into_iter()
                            .flat_map(|c| c.0.iter())
                            .enumerate()
                            .filter_map(|(r, cell)| {
                                let yes = if negative && cell.0 == super::finish::NULL {
                                    false
                                } else if spec.kinds[*col] == languages::CellKind::Span {
                                    let span = (cell.0 >> 32) as usize..(cell.0 as u32) as usize;
                                    self.src.get(span).is_some_and(|text| regex.is_match(text))
                                } else {
                                    self.matches.test(self.p, *re, *cell, self.words)
                                };
                                (yes != negative)
                                    .then(|| key.map_or(0, |c| c.0[r].0 as u32))
                                    .filter(|x| spec.key.is_none() || *x < n)
                            }),
                    ))
                }
            }
            Domain::Fact(_, Some(_)) => self.posted(id, false),
            Domain::Fact(slot, None) => {
                let spec = &self.p.facts[*slot];
                if !self.facts[*slot].read && spec.key.is_some() {
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
                    Some(
                        self.arena
                            .bits((0..table.cells.0.first().map_or(0, |c| c.0.len())).map(node)),
                    )
                }
            }
        };
        self.sets.set(id, nodes.clone());
        nodes
    }
}

impl Program {
    pub(super) fn eval(
        &self,
        scratch: &mut Scratch,
        order: &Schedule,
        t: &NodeTable,
        src: &[u8],
        path: Option<&[u8]>,
        syntax_text: Option<SyntaxText<'_>>,
        words: &mut FileWords,
        read: impl FnMut(Source, Query<'_>, &mut FileWords, &mut FilledColumns) -> Option<bool>,
        alive: &mut [bool],
        exact: Option<&mut Vec<(usize, Vec<u64>)>>,
    ) -> u64 {
        #[cfg(feature = "pair-diagnostics")]
        if self.meter.is_some() {
            return self.evaluate::<true>(scratch, order, t, src, path, syntax_text, words, read, alive, exact);
        }
        self.evaluate::<false>(scratch, order, t, src, path, syntax_text, words, read, alive, exact)
    }

    fn evaluate<const PROFILE: bool>(
        &self,
        scratch: &mut Scratch,
        order: &Schedule,
        t: &NodeTable,
        src: &[u8],
        path: Option<&[u8]>,
        syntax_text: Option<SyntaxText<'_>>,
        words: &mut FileWords,
        mut read: impl FnMut(Source, Query<'_>, &mut FileWords, &mut FilledColumns) -> Option<bool>,
        alive: &mut [bool],
        mut exact: Option<&mut Vec<(usize, Vec<u64>)>>,
    ) -> u64 {
        languages::facts::clear_words(words);
        scratch.arena.words.clear();
        scratch.sets.reset(self.domains.len());
        scratch.needs.clear();
        scratch.needs.resize(self.needs.len(), None);
        scratch.facts.resize_with(self.facts.len(), Table::default);
        for t in &mut scratch.facts {
            t.read = false;
            t.answered = false;
            t.complete = false;
            for c in &mut t.cells.0 {
                c.0.clear();
            }
        }
        scratch.matches.reset(
            self.regexes.len(),
            languages::facts::base_count(words).0 as usize,
        );
        scratch.witnesses.clear();
        scratch.witnesses.resize(self.domains.len(), None);
        scratch.postings.reset(&order.postings);
        let reader_class = std::cell::Cell::new(cost::Family::Logic as usize);
        let mut observe = |source, query: Query<'_>, words: &mut FileWords, cells: &mut FilledColumns| {
            if !PROFILE {
                return read(source, query, words, cells);
            }
            let start = super::thread_cpu();
            let ok = read(source, query, words, cells);
            let elapsed = super::thread_cpu() - start;
            let slot = self
                .facts
                .iter()
                .position(|f| f.source == source)
                .expect("declared source");
            let meter = self.meter.as_ref().unwrap();
            let mut fills = meter.fills.borrow_mut();
            let entry = &mut fills[slot][reader_class.get()];
            entry.0 += 1;
            entry.1 += cells.0.first().map_or(0, |c| c.0.len() as u64);
            entry.2 += elapsed - meter.overhead;
            ok
        };
        let mut f = File::<PROFILE> {
            reader_class: &reader_class,
            owner: 0,
            file_owner: if PROFILE {
                vec![usize::MAX; self.needs.len()]
            } else {
                vec![]
            },
            node_owner: if PROFILE {
                vec![usize::MAX; self.domains.len()]
            } else {
                vec![]
            },
            plain_tree: !self.paths.is_empty() && node_table::embedded_segments(t).next().is_none(),
            charged: 0.,
            stats: Default::default(),
            overhead: self.meter.as_ref().map_or(0., |m| m.overhead),
            postings: &mut scratch.postings,
            literals: &mut scratch.literals,
            byte_tests: Map::default(),
            witnesses: &mut scratch.witnesses,
            matches: &mut scratch.matches,
            p: self,
            t,
            src,
            syntax_text,
            path,
            read: &mut observe,
            words,
            facts: &mut scratch.facts,
            arena: &mut scratch.arena,
            order,
            sets: &mut scratch.sets,
            needs: &mut scratch.needs,
            steps: 0,
        };
        if PROFILE {
            for (ri, (active, (_, root))) in alive.iter_mut().zip(&self.roots).enumerate() {
                f.owner = ri;
                if *active { *active = f.pass(*root); }
            }
        } else {
        order
            .rules
            .apply(alive, &mut scratch.alive_words, |id| f.pass(id));
        }
        if let Some(out) = exact.as_mut() {
            for (rule, cert) in &self.exact {
                let rule = *rule;
                if PROFILE {
                    f.owner = rule;
                }
                if alive[rule] && !f.pass(cert.guard) && f.complete(&cert.facts) {
                    let nodes = f
                        .nodes(cert.nodes)
                        .expect("certified domains contain only total node tests");
                    out.push((rule, f.arena.absolute(nodes)));
                }
            }
        }
        if PROFILE {
            let meter = self.meter.as_ref().unwrap();
            for (a, b) in meter.rows.borrow_mut().iter_mut().zip(f.stats) {
                a.calls += b.calls;
                a.hits += b.hits;
                a.cross += b.cross;
                a.ns += b.ns;
            }
            for (n, t) in meter.loads.borrow_mut().iter_mut().zip(f.facts.iter()) {
                *n += u64::from(t.read);
            }
        }
        #[cfg(feature = "summary-diagnostics")]
        {
            scratch.words += f.arena.words.len() as u64;
            scratch.peak = scratch.peak.max(f.arena.words.len());
        }
        f.steps
    }
}
