//! Evaluate each requested witness once per file. Unknown or partial provider answers admit the file.
use super::pair_summary::cost::{self, Sample};
use super::pair_summary::{Domain, Id, Need, Program, Source};
use rustc_hash::FxHashMap as Map;
use ryo::languages::{self, FileWords, FilledColumns, NodeTable, Position, node_table};
use std::rc::Rc;

#[path = "pair_counts.rs"]
mod counts;
#[path = "pair_witness.rs"]
mod witness;

type Bits = Rc<Vec<u64>>;
// None is the unconstrained node set. An empty vector is the empty set.
type Nodes = Option<Bits>;
type SymbolColumn<'a> = dyn Fn(Source, usize, &super::pair_summary::regexes::Matcher) -> Option<bool> + 'a;
/// The exact syntax text column supplied by a host with per-position symbol IDs.
#[derive(Clone, Copy)]
pub(super) struct SyntaxText<'a> {
    pub ids: &'a [u32],
    pub symbols: &'a [std::rc::Rc<[u8]>],
    pub patterns: Option<&'a SymbolMatches>,
    pub dictionary: &'a Map<Rc<[u8]>, u32>,
    pub distinct: Option<&'a std::cell::RefCell<FileSymbols>>,
    pub column: Option<&'a SymbolColumn<'a>>,
}
/// The reader owns symbol identities for the lifetime of this dictionary.
#[derive(Default)]
pub(super) struct SymbolMatches(std::cell::RefCell<Vec<(usize, u32, bool)>>);

/// An existence test can stop early. Later predicates continue the same
/// discovery of distinct subjects and see every subject already discovered.
#[derive(Default)]
pub(super) struct FileSymbols {
    seen: Vec<u64>,
    words: Vec<u32>,
    next: usize,
}
impl FileSymbols {
    #[allow(dead_code, reason = "session readers reuse this index across source versions")]
    pub(super) fn clear(&mut self) {
        for word in self.words.drain(..) {
            self.seen[word as usize / 64] = 0;
        }
        self.next = 0;
    }

    fn any(&mut self, column: &[u32], mut accepts: impl FnMut(u32) -> bool) -> bool {
        if self.words.iter().copied().any(&mut accepts) {
            return true;
        }
        while self.next < column.len() {
            let id = column[self.next];
            self.next += 1;
            let block = id as usize / 64;
            if self.seen.len() <= block {
                self.seen.resize(block + 1, 0);
            }
            let bit = 1u64 << (id % 64);
            if self.seen[block] & bit == 0 {
                self.seen[block] |= bit;
                self.words.push(id);
                if accepts(id) {
                    return true;
                }
            }
        }
        false
    }
}

impl SymbolMatches {
    fn with_predicate<R>(&self, index: usize, regex: &super::pair_summary::regexes::Matcher,
        symbols: &[Rc<[u8]>], apply: impl FnOnce(&mut dyn FnMut(u32) -> bool) -> R) -> R {
        let mut entries = self.0.borrow_mut();
        if entries.is_empty() {
            entries.resize(4096, (usize::MAX, 0, false));
        }
        apply(&mut |id| {
            let slot = (index.wrapping_mul(0x9e3779b9) ^ id as usize) & (entries.len() - 1);
            let entry = &mut entries[slot];
            if (entry.0, entry.1) != (index, id) {
                *entry = (index, id, regex.is_match(&symbols[id as usize]));
            }
            entry.2
        })
    }
}
impl SyntaxText<'_> {
    #[inline(never)]
    fn matches(self, index: usize, regex: &super::pair_summary::regexes::Matcher, polarity: bool) -> bool {
        if let Some(patterns) = self.patterns {
            patterns.with_predicate(index, regex, self.symbols, |test|
                if let Some(distinct) = self.distinct {
                    distinct.borrow_mut().any(self.ids, |id| test(id) == polarity)
                } else {
                    self.ids.iter().any(|&id| test(id) == polarity)
                })
        } else {
            self.ids.iter().any(|&id| regex.is_match(&self.symbols[id as usize]) == polarity)
        }
    }
}
impl<'a> SyntaxText<'a> {
    fn at(self, index: usize) -> &'a [u8] {
        &self.symbols[self.ids[index] as usize]
    }
}
type Reader<'a> = dyn FnMut(Source, &mut FileWords, &mut FilledColumns) -> bool + 'a;

#[derive(Default)]
struct Table {
    read: bool,
    answered: bool,
    complete: bool,
    cells: FilledColumns,
    index: Map<usize, Map<Id, Vec<u32>>>,
}
/// Empty between files. Only materialized node masks need clearing.
#[derive(Default)]
pub(super) struct Scratch {
    sets: Vec<Option<Nodes>>,
    needs: Vec<Option<bool>>,
    touched: Vec<Id>,
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
    p: &'a Program,
    t: &'a NodeTable,
    src: &'a [u8],
    syntax_text: Option<SyntaxText<'a>>,
    path: Option<&'a [u8]>,
    read: &'a mut Reader<'a>,
    words: &'a mut FileWords,
    facts: Vec<Table>,
    sets: &'a mut [Option<Nodes>],
    needs: &'a mut [Option<bool>],
    touched: &'a mut Vec<Id>,
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
fn bits(ids: impl IntoIterator<Item = u32>) -> Bits {
    let mut b = Vec::new();
    for n in ids {
        insert(&mut b, n)
    }
    Rc::new(b)
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
        let answered = read(spec.source, words, &mut self.cells);
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
            Need::PathMatches(re,yes) => self.path.is_none_or(|p|
                self.p.regexes[re].compiled().is_none_or(|r| r.is_match(p) == yes)),
            Need::SyntaxMatches(re,yes) => self.syntax_text.is_none_or(|col|{
                self.p.regexes[re].compiled().is_none_or(|r|
                    col.matches(re, r, yes))
            }),
            Need::Terms(all, ref terms) => {
                if all {
                    terms.iter().all(|i| self.pass(*i))
                } else {
                    terms.iter().any(|i| self.pass(*i))
                }
            }
            Need::Both(a, b) => self.pass(a) && self.pass(b),
            Need::Either(a, b) => self.pass(a) || self.pass(b),
            Need::Some(x) => match &self.p.domains[x] {
                // All stored bitsets end at their last nonzero word. A file
                // prerequisite can stop at its first witness without allocating
                // the intersection or union of every matching node.
                _ if self.sets[x].is_some() => self.sets[x]
                    .as_ref()
                    .unwrap()
                    .as_ref()
                    .is_none_or(|b| !b.is_empty()),
                _ if self.plain_tree && self.p.paths[x].is_some_and(|e| e.existence_only) => {
                    let edge = self.p.paths[x].unwrap();
                    self.edge(edge, None)
                        .unwrap_or_else(|| self.nodes(x).is_none_or(|b| !b.is_empty()))
                }
                _ if self.plain_tree && self.p.paths[x].is_some() => {
                    self.nodes(x).is_none_or(|b| !b.is_empty())
                }
                Domain::Parents(a) | Domain::Ancestors(a)
                    if self.plain_tree && self.p.file_only[x] =>
                {
                    self.path_exists(*a, false)
                }
                Domain::Children(a) | Domain::Descendants(a)
                    if self.plain_tree && self.p.file_only[x] =>
                {
                    self.path_exists(*a, true)
                }
                Domain::Both(a, b) => {
                    let l = self.nodes(*a);
                    if l.as_ref().is_some_and(|v| v.is_empty()) {
                        false
                    } else {
                        match (l, self.nodes(*b)) {
                            (Some(l), Some(r)) => l.iter().zip(r.iter()).any(|(a, b)| a & b != 0),
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
                Domain::Text(literal) if self.syntax_text.is_some() => {
                    let column = self.syntax_text.unwrap();
                    column.dictionary.get(literal.as_bytes()).is_some_and(|id| column.ids.contains(id))
                }
                Domain::SpanMatching(kind, pattern) if self.p.file_only[x] => {
                    node_table::ids_of_kind(self.t, languages::KindBits(*kind)).iter().any(|pos| {
                        self.p.regexes[*pattern].is_match(&self.src[super::span_bytes(self.t, pos.0 as usize)])
                    })
                }
                Domain::TextMatching(kind, pattern) if self.p.file_only[x] && self.syntax_text.is_some() => {
                    let col = self.syntax_text.unwrap();
                    let ids = node_table::ids_of_kind(self.t, languages::KindBits(*kind));
                    match (self.p.regexes[*pattern].compiled(), col.patterns) {
                        (Some(regex), Some(cache)) => cache.with_predicate(*pattern, regex, col.symbols, |test|
                            ids.iter().any(|p| test(col.ids[p.0 as usize]))),
                        _ => ids.iter().any(|p| self.p.regexes[*pattern].is_match(col.at(p.0 as usize))),
                    }
                }
                Domain::Matching(slot, column, pattern)
                    if self.syntax_text.is_some_and(|view| view.column.is_some())
                        && self.p.facts[*slot].kinds[*column] == languages::CellKind::Symbol =>
                {
                    let view = self.syntax_text.unwrap();
                    let found = self.p.regexes[*pattern].compiled().and_then(|p|
                        (view.column.unwrap())(self.p.facts[*slot].source, *column, p));
                    found.unwrap_or_else(|| self.nodes(x).is_none_or(|bits| !bits.is_empty()))
                }
                Domain::Fact(slot, None) => {
                    let table = &mut self.facts[*slot];
                    if !table.read && self.p.facts[*slot].key.is_some() {
                        // A bare file-level existence guard alone cannot justify building
                        // a keyed table. A subsequent constrained check can still request it.
                        self.needs[id] = Some(true);
                        return true;
                    }
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
    fn nodes(&mut self, id: Id) -> Nodes {
        let class = if PROFILE {
            cost::family(&self.p.domains[id], &self.p.facts) as usize
        } else {
            0
        };
        if PROFILE {
            self.stats[class].calls += 1;
        }
        if let Some(result) = &self.sets[id] {
            if PROFILE {
                self.stats[class].hits += 1;
                self.stats[class].cross += u64::from(self.node_owner[id] != self.owner);
            }
            return result.clone();
        }
        self.touched.push(id);
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
        self.steps += 1;
        let n =
            u32::try_from(node_table::element_count(self.t).0).expect("table positions fit u32");
        if self.plain_tree
            && let Some(edge) = self.p.paths[id]
        {
            let mut output = Vec::new();
            if self.edge(edge, Some(&mut output)).is_some() {
                let nodes = Some(Rc::new(output));
                self.sets[id] = Some(nodes.clone());
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
                    children,
                    matches!(self.p.domains[rows], Domain::Any),
                    depth,
                    op,
                    value,
                )
            }
            Domain::HasParent => {
                Some(bits((1..n).filter(|&i| {
                    node_table::parent_of(self.t, Position(i)).0 != i
                })))
            }
            Domain::Empty => Some(Rc::default()),
            Domain::Kind(k) => Some(bits(
                node_table::ids_of_kind(self.t, languages::KindBits(*k))
                    .iter()
                    .map(|p| p.0),
            )),
            Domain::Field(k) => Some(bits(
                node_table::ids_of_field(self.t, languages::FieldCode(*k))
                    .iter()
                    .map(|p| p.0),
            )),
            Domain::SpanMatching(k, re) => Some(bits(
                node_table::ids_of_kind(self.t, languages::KindBits(*k))
                    .iter()
                    .filter_map(|pos| {
                        let span = super::span_bytes(self.t, pos.0 as usize);
                        self.p.regexes[*re]
                            .is_match(&self.src[span])
                            .then_some(pos.0)
                    }),
            )),
            Domain::TextMatching(k, re) => {
                let regex = &self.p.regexes[*re];
                if let Some(column) = self.syntax_text
                    && let Some(cache) = column.patterns
                    && let Some(matcher) = regex.compiled()
                {
                    let nodes = cache.with_predicate(*re, matcher, column.symbols, |test| {
                        bits(node_table::ids_of_kind(self.t, languages::KindBits(*k)).iter()
                            .filter(|p| test(column.ids[p.0 as usize])).map(|p| p.0))
                    });
                    self.sets[id] = Some(Some(Rc::clone(&nodes)));
                    return Some(nodes);
                }
                Some(bits(
                    node_table::ids_of_kind(self.t, languages::KindBits(*k))
                        .iter()
                        .filter_map(|p| {
                            let text = self.syntax_text.map_or_else(
                                || node_table::decoded_of(self.t, *p).map_or_else(
                                    || &self.src[super::span_bytes(self.t, p.0 as usize)],
                                    |t| t.0.as_bytes(),
                                ),
                                |column| column.at(p.0 as usize),
                            );
                            regex.is_match(text).then_some(p.0)
                        }),
                ))
            }
            Domain::Text(t) => {
                let by_text = self.texts.get_or_insert_with(|| {
                    let mut m = Map::<&[u8], Vec<u32>>::default();
                    if let Some(column) = self.syntax_text {
                        let wanted: Map<_, _> = self.p.texts.iter().filter_map(|s|
                            column.dictionary.get(s.as_slice()).map(|&id| (id, s.as_slice()))).collect();
                        for (i, id) in column.ids.iter().enumerate() {
                            if let Some(&text) = wanted.get(id) {
                                m.entry(text).or_default().push(i as u32);
                            }
                        }
                        return m;
                    }
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
                Some(bits(
                    by_text.get(t.as_bytes()).into_iter().flatten().copied(),
                ))
            }
            Domain::Both(a, b) => {
                let l = self.nodes(*a);
                if l.as_ref().is_some_and(|b| b.is_empty()) {
                    l
                } else {
                    match (l, self.nodes(*b)) {
                        (Some(a), Some(b)) => {
                            let mut v: Vec<_> =
                                a.iter().zip(b.iter()).map(|(x, y)| x & y).collect();
                            while v.last() == Some(&0) {
                                v.pop();
                            }
                            Some(Rc::new(v))
                        }
                        (a, b) => a.or(b),
                    }
                }
            }
            Domain::When(need, nodes) => {
                if self.pass(*need) {
                    self.nodes(*nodes)
                } else {
                    Some(Rc::default())
                }
            }
            // The excluded side must be exact in this particular file.
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
                    self.sets[id] = Some(left.clone());
                    return left;
                }
                let Some(right) = self.nodes(*b) else {
                    // A missing provider answer cannot prove an exclusion.
                    let out = if matches!(self.p.domains[*b], Domain::Any) {
                        Some(Rc::default())
                    } else {
                        left
                    };
                    self.sets[id] = Some(out.clone());
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
                    |v| (*v).clone(),
                );
                for (w, r) in out.iter_mut().zip(right.iter()) {
                    *w &= !*r
                }
                while out.last() == Some(&0) {
                    out.pop();
                }
                Some(Rc::new(out))
            }
            // An unconstrained arm admits every node. Only a known left mask
            // can justify requesting the right arm and its provider tables.
            Domain::Either(a, b) => self.nodes(*a).and_then(|left| {
                let right = self.nodes(*b)?;
                let (a, b) = if left.len() >= right.len() {
                    (left, right)
                } else {
                    (right, left)
                };
                let mut v = (*a).clone();
                for (x, y) in v.iter_mut().zip(b.iter()) {
                    *x |= *y;
                }
                Some(Rc::new(v))
            }),
            Domain::Descendants(root) => self.nodes(*root).map(|ancestors| {
                let mut pending = Vec::new();
                each(&ancestors, |n| pending.push(n));
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
                Rc::new(seen)
            }),
            Domain::Parents(x) | Domain::Children(x) | Domain::Ancestors(x) => {
                let children = matches!(self.p.domains[id], Domain::Children(_));
                let ancestors = matches!(self.p.domains[id], Domain::Ancestors(_));
                let input = if matches!(self.p.domains[*x], Domain::Any) {
                    Some(bits(0..n))
                } else {
                    self.nodes(*x)
                };
                input.map(|b| {
                    let mut out = Vec::new();
                    each(&b, |i| {
                        if children {
                            for c in node_table::children_of(self.t, Position(i)) {
                                insert(&mut out, c.0)
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
                                insert(&mut out, p);
                                if !ancestors {
                                    break;
                                }
                                child = p;
                            }
                        }
                    });
                    Rc::new(out)
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
                        let nodes = bits(keys.0.iter().enumerate().filter_map(|(i, k)| {
                            let keep = values.is_none_or(|(c, ids)| {
                                let text = languages::facts::text(self.words, c.0[i]).0;
                                self.p
                                    .symbols
                                    .get(text)
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
                    Some(bits(
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
                    ))
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
                    Some(bits((0..len).filter_map(|row| {
                        let matched = tests.iter().all(|test| match test {
                            CellTest::Symbols(c, values) => {
                                let cell = table.cells.0[*c].0[row];
                                cell.0 != super::finish::NULL
                                    && self
                                        .p
                                        .symbols
                                        .get(languages::facts::text(self.words, cell).0)
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
                    Some(bits(
                        table
                            .cells
                            .0
                            .get(*col)
                            .into_iter()
                            .flat_map(|c| c.0.iter())
                            .enumerate()
                            .filter_map(|(i, cell)| {
                                let text = languages::facts::text(self.words, *cell).0;
                                self.p
                                    .symbols
                                    .get(text)
                                    .is_none_or(|id| excluded.binary_search(id).is_err())
                                    .then(|| key.map_or(0, |ids| ids.0[i].0 as u32))
                            }),
                    ))
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
                    Some(bits(
                        table
                            .cells
                            .0
                            .get(*col)
                            .into_iter()
                            .flat_map(|c| c.0.iter())
                            .enumerate()
                            .filter_map(|(i, cell)| {
                                let text = languages::facts::text(self.words, *cell).0.as_bytes();
                                text.windows(delimiter.len())
                                    .enumerate()
                                    .any(|(p, w)| {
                                        w == delimiter
                                            && regex.is_match(&text[p + delimiter.len()..])
                                    })
                                    .then(|| key.map_or(0, |ids| ids.0[i].0 as u32))
                            }),
                    ))
                }
            }
            Domain::Member(slot, col, project, field) => {
                let (slot, col, project, field) = (*slot, *col, *project, *field);
                let available = &mut self.facts[project];
                available.load(&self.p.facts[project], n, self.words, self.read);
                if !available.complete {
                    None
                } else if available.cells.0.first().is_none_or(|c| c.0.is_empty()) {
                    Some(Rc::default())
                } else {
                    let allowed: Vec<_> = available.cells.0[field].0.iter().map(|v| v.0).collect();
                    let t = &mut self.facts[slot];
                    let spec = &self.p.facts[slot];
                    t.load(spec, n, self.words, self.read);
                    if !t.complete {
                        None
                    } else {
                        let key = spec.key.and_then(|k| t.cells.0.get(k));
                        Some(bits(
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
                        self.sets[id] = Some(None);
                        return None;
                    };
                    Some(bits(
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
                                    regex.is_match(
                                        languages::facts::text(self.words, *cell).0.as_bytes(),
                                    )
                                };
                                (yes != negative)
                                    .then(|| key.map_or(0, |c| c.0[r].0 as u32))
                                    .filter(|x| spec.key.is_none() || *x < n)
                            }),
                    ))
                }
            }
            Domain::Fact(slot, test) => {
                let spec = &self.p.facts[*slot];
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
                                    let text = languages::facts::text(self.words, cell);
                                    if let Some(&id) = self.p.symbols.get(text.0) {
                                        m.entry(id).or_default().push(node(r));
                                    }
                                }
                            }
                            m
                        });
                        Some(bits(
                            values.iter().filter_map(|v| idx.get(v)).flatten().copied(),
                        ))
                    } else {
                        Some(bits(
                            (0..table.cells.0.first().map_or(0, |c| c.0.len()))
                                .map(node)
                                .filter(|x| spec.key.is_none() || *x < n),
                        ))
                    }
                }
            }
        };
        self.sets[id] = Some(nodes.clone());
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
        read: impl FnMut(Source, &mut FileWords, &mut FilledColumns) -> bool,
        alive: &mut [bool],
        exact: Option<&mut Vec<(usize, Bits)>>,
    ) -> u64 {
        self.file_columns(t, src, path, None, words, read, alive, exact, &[])
    }
    pub(super) fn file_columns(
        &self,
        t: &NodeTable,
        src: &[u8],
        path: Option<&[u8]>,
        syntax_text: Option<SyntaxText<'_>>,
        words: &mut FileWords,
        read: impl FnMut(Source, &mut FileWords, &mut FilledColumns) -> bool,
        alive: &mut [bool],
        exact: Option<&mut Vec<(usize, Bits)>>,
        known: &[(Id, bool)],
    ) -> u64 {
        #[cfg(feature = "pair-diagnostics")]
        if self.meter.is_some() {
            return self.evaluate::<true>(t, src, path, syntax_text, words, read, alive, exact, known);
        }
        self.evaluate::<false>(t, src, path, syntax_text, words, read, alive, exact, known)
    }
    fn evaluate<const PROFILE: bool>(
        &self,
        t: &NodeTable,
        src: &[u8],
        path: Option<&[u8]>,
        syntax_text: Option<SyntaxText<'_>>,
        words: &mut FileWords,
        mut read: impl FnMut(Source, &mut FileWords, &mut FilledColumns) -> bool,
        alive: &mut [bool],
        mut exact: Option<&mut Vec<(usize, Bits)>>,
        known: &[(Id, bool)],
    ) -> u64 {
        languages::facts::clear_words(words);
        let reader_class = std::cell::Cell::new(cost::Family::Logic as usize);
        let mut observe = |source, words: &mut FileWords, cells: &mut FilledColumns| {
            if !PROFILE {
                return read(source, words, cells);
            }
            let start = super::thread_cpu();
            let ok = read(source, words, cells);
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
        let mut buffers = self.scratch.take();
        buffers.sets.resize(self.domains.len(), None);
        buffers.needs.resize(self.needs.len(), None);
        buffers.needs.fill(None);
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
            p: self,
            t,
            src,
            syntax_text,
            path,
            read: &mut observe,
            words,
            facts: (0..self.facts.len()).map(|_| Table::default()).collect(),
            sets: &mut buffers.sets,
            needs: &mut buffers.needs,
            touched: &mut buffers.touched,
            texts: None,
            steps: 0,
        };
        for &(need, value) in known { f.needs[need] = Some(value); }
        for (rule, (a, (_, root))) in alive.iter_mut().zip(&self.roots).enumerate() {
            if PROFILE {
                f.owner = rule;
            }
            if *a {
                *a = f.pass(*root)
            }
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
                    out.push((rule, nodes));
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
            for (n, t) in meter.loads.borrow_mut().iter_mut().zip(&f.facts) {
                *n += u64::from(t.read);
            }
        }
        let steps = f.steps;
        drop(f);
        for id in buffers.touched.drain(..) {
            buffers.sets[id] = None;
        }
        self.scratch.replace(buffers);
        steps
    }
}
