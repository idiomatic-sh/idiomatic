//! Evaluate each requested witness once per file. Unknown or partial provider answers admit the file.
use super::pair_order::{Bucket, Order};
use super::pair_summary::{Domain, Id, Need, Program};
use rustc_hash::FxHashMap as Map;
use ryo::languages::{self, FileWords, FilledColumns, NodeTable, Position, Relation, node_table};
#[path = "pair_arena.rs"]
mod arena;
use arena::{Arena, Bits};
#[path = "pair_edge.rs"]
pub(super) mod edge;
#[path = "pair_inputs.rs"]
pub(super) mod inputs;
#[path = "pair_mask.rs"]
pub(super) mod mask;
#[path = "pair_presence.rs"]
pub(super) mod presence;
#[path = "pair_probe.rs"]
mod probe;
#[path = "pair_regex.rs"]
pub(super) mod regex_group;
#[path = "pair_symbols.rs"]
mod symbol_codes;

// None is the unconstrained node set. An empty vector is the empty set.
type Nodes = Option<Bits>;
pub(super) enum Request<'a> {
    Rows(languages::Wanted),
    Has(Option<Position>),
    Keys(usize, &'a mut Vec<u64>),
    Symbols {
        key: usize,
        column: usize,
        index: &'a mut Map<Id, Vec<u32>>,
    },
}
type Reader<'a> =
    dyn FnMut(Relation, Request<'_>, &mut FileWords, &mut FilledColumns) -> Option<bool> + 'a;

#[derive(Default)]
struct Table {
    read: bool,
    complete: bool,
    cells: FilledColumns,
    by_node: Map<u32, Option<FilledColumns>>,
    index: Map<usize, Map<Id, Vec<u32>>>,
}
#[derive(Default)]
pub(super) struct Work {
    pub steps: u64,
    pub candidates: u64,
    pub local_tests: u64,
    pub bucket_requests: u64,
    pub distinct_buckets: usize,
}
#[derive(Default)]
pub(super) struct Scratch {
    codes: symbol_codes::Codes,
    pub counts: Vec<u64>,
    pub peak_words: usize,
    pub total_words: u64,
    arena: Arena,
    facts: Vec<Table>,
    sets: Vec<Option<Nodes>>,
    needs: Vec<Option<bool>>,
}
struct File<'a> {
    p: &'a Program,
    order: Option<&'a Order>,
    t: &'a NodeTable,
    src: &'a [u8],
    path: Option<&'a [u8]>,
    read: &'a mut Reader<'a>,
    words: &'a mut FileWords,
    arena: &'a mut Arena,
    codes: &'a mut symbol_codes::Codes,
    counts: &'a mut [u64],
    facts: &'a mut [Table],
    sets: &'a mut [Option<Nodes>],
    needs: &'a mut [Option<bool>],
    texts: Option<Map<&'a [u8], Vec<u32>>>,
    steps: u64,
    candidates: u64,
    node_tests: u64,
    local: Map<(Id, u32), (u64, u64)>,
    edges: edge::Answers,
    bucket_texts: Map<u16, inputs::BucketText<'a>>,
    absent: Vec<u64>,
    bucket_requests: u64,
    visited_buckets: rustc_hash::FxHashSet<Bucket>,
}

fn insert(bits: &mut Vec<u64>, n: u32) {
    let w = (n / 64) as usize;
    if bits.len() <= w {
        bits.resize(w + 1, 0)
    }
    bits[w] |= 1 << (n % 64);
}
fn each(b: &[u64], mut visit: impl FnMut(u32)) {
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
        self.complete = read(
            spec.relation,
            Request::Rows(languages::Wanted::Whole),
            words,
            &mut self.cells,
        )
        .expect("row fill answered");
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
        if self.order.is_some_and(|o| o.costs[b] < o.costs[a]) {
            (b, a)
        } else {
            (a, b)
        }
    }
    fn pass(&mut self, id: Id) -> bool {
        if let Some(x) = self.needs[id] {
            return x;
        }
        self.steps += 1;
        if self.missing(id, false) {
            self.needs[id] = Some(false);
            return false;
        }
        let yes = match self.p.needs[id] {
            Need::Any => true,
            Need::Empty => false,
            Need::PathMatches(re) => self
                .path
                .is_none_or(|path| self.p.regexes[re].is_match(path)),
            Need::Both(..) if self.order.is_some() => {
                self.order.unwrap().groups[id].iter().all(|&x| self.pass(x))
            }
            Need::Either(..) if self.order.is_some() => {
                self.order.unwrap().groups[id].iter().any(|&x| self.pass(x))
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
                _ if self.order.is_some()
                    && let Some(value) = self.edge_witness(x) =>
                {
                    value
                }
                Domain::Both(..) | Domain::TextMatching(..)
                    if self.order.is_some_and(|o| o.sparse)
                        && let Some(value) = self.sparse_witness(x) =>
                {
                    value
                }
                _ if self.order.is_some_and(|o| !o.masks.is_empty())
                    && let Some(yes) = self.masked_witness(x) =>
                {
                    yes
                }
                Domain::Both(a, b) => {
                    let (a, b) = self.first(*a, *b);
                    let l = self.nodes(a);
                    if l.as_ref().is_some_and(|v| v.is_empty()) {
                        false
                    } else {
                        match (l, self.nodes(b)) {
                            (Some(l), Some(r)) => self.arena.intersects(l, r),
                            (l, r) => l.or(r).is_none_or(|v| !v.is_empty()),
                        }
                    }
                }
                Domain::Either(a, b) => {
                    let (a, b) = self.first(*a, *b);
                    self.nodes(a).is_none_or(|v| !v.is_empty())
                        || self.nodes(b).is_none_or(|v| !v.is_empty())
                }
                Domain::Kind(k) => {
                    !node_table::ids_of_kind(self.t, languages::KindBits(*k)).is_empty()
                }
                Domain::Field(k) => {
                    node_table::field_count(self.t, languages::FieldCode(*k)).0 != 0
                }
                Domain::Fact(slot, None) => {
                    if self.order.is_some()
                        && !self.facts[*slot].read
                        && let Some(yes) = (self.read)(
                            self.p.facts[*slot].relation,
                            Request::Has(None),
                            self.words,
                            &mut FilledColumns::default(),
                        )
                    {
                        self.needs[id] = Some(yes);
                        return yes;
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
    fn index_texts(&mut self) {
        if self.literal_postings() {
            return;
        }
        self.texts.get_or_insert_with(|| {
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
    }
    fn nodes(&mut self, id: Id) -> Nodes {
        if let Some(s) = &self.sets[id] {
            return s.clone();
        }
        if !self.counts.is_empty() {
            self.counts[id] += 1;
        }
        self.steps += 1;
        if self.missing(id, true) {
            let empty = Some(Bits::default());
            self.sets[id] = Some(empty);
            return empty;
        }
        let n =
            u32::try_from(node_table::element_count(self.t).0).expect("table positions fit u32");
        if let Some(mask) = self.input_mask(id) {
            return mask;
        }
        if matches!(
            self.p.domains[id],
            Domain::TextMatching(..) | Domain::Matching(..)
        ) && let Some(set) = self.grouped_matches(id)
        {
            return set;
        }
        let nodes = match &self.p.domains[id] {
            Domain::Any => None,
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
            Domain::TextMatching(k, re) => {
                let regex = &self.p.regexes[*re];
                Some(
                    self.arena.bits(
                        node_table::ids_of_kind(self.t, languages::KindBits(*k))
                            .iter()
                            .filter_map(|p| {
                                let text = node_table::decoded_of(self.t, *p).map_or_else(
                                    || &self.src[super::span_bytes(self.t, p.0 as usize)],
                                    |t| t.0.as_bytes(),
                                );
                                regex.is_match(text).then_some(p.0)
                            }),
                    ),
                )
            }
            Domain::Text(t) => {
                self.index_texts();
                let by_text = self.texts.as_ref().unwrap();
                Some(
                    self.arena
                        .bits(by_text.get(t.as_bytes()).into_iter().flatten().copied()),
                )
            }
            Domain::Both(a, b) => {
                let (a, b) = self.first(*a, *b);
                let l = self.nodes(a);
                if l.as_ref().is_some_and(|b| b.is_empty()) {
                    l
                } else {
                    match (l, self.nodes(b)) {
                        (Some(a), Some(b)) => Some(self.arena.intersect(a, b)),
                        (a, b) => a.or(b),
                    }
                }
            }
            Domain::Either(a, b) => match (self.nodes(*a), self.nodes(*b)) {
                (Some(a), Some(b)) => Some(self.arena.union(a, b)),
                _ => None,
            },
            Domain::Parents(x) | Domain::Children(x) | Domain::Ancestors(x) => {
                let children = matches!(self.p.domains[id], Domain::Children(_));
                let ancestors = matches!(self.p.domains[id], Domain::Ancestors(_));
                self.nodes(*x).map(|b| {
                    let out = &mut self.arena.work;
                    each(&self.arena.words[b.range()], |i| {
                        if children {
                            for c in node_table::children_of(self.t, Position(i)) {
                                insert(out, c.0)
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
            Domain::Fact(slot, test) => {
                let spec = &self.p.facts[*slot];
                if self.order.is_some()
                    && test.is_none()
                    && !self.facts[*slot].read
                    && let Some(key) = spec.key
                {
                    let mut keys = Vec::new();
                    if let Some(complete) = (self.read)(
                        spec.relation,
                        Request::Keys(key, &mut keys),
                        self.words,
                        &mut FilledColumns::default(),
                    ) {
                        if !complete {
                            self.sets[id] = Some(None);
                            return None;
                        }
                        while keys.last() == Some(&0) {
                            keys.pop();
                        }
                        let out = Some(self.arena.commit(&keys));
                        self.sets[id] = Some(out.clone());
                        return out;
                    }
                }
                if self.order.is_some()
                    && let (Some(key), Some((col, values))) = (spec.key, test)
                    && !self.facts[*slot].read
                {
                    let table = &mut self.facts[*slot];
                    if !table.index.contains_key(col) {
                        let mut index = Map::default();
                        let answer = (self.read)(
                            spec.relation,
                            Request::Symbols {
                                key,
                                column: *col,
                                index: &mut index,
                            },
                            self.words,
                            &mut FilledColumns::default(),
                        );
                        match answer {
                            Some(true) => {
                                table.index.insert(*col, index);
                            }
                            Some(false) => {
                                self.sets[id] = Some(None);
                                return None;
                            }
                            None => {}
                        }
                    }
                    if let Some(index) = table.index.get(col) {
                        let set = Some(
                            self.arena.bits(
                                values
                                    .iter()
                                    .filter_map(|v| index.get(v))
                                    .flatten()
                                    .copied(),
                            ),
                        );
                        self.sets[id] = Some(set.clone());
                        return set;
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
                        let integer = self.order.is_some_and(|o| o.codes);
                        if integer {
                            self.codes.extend(self.words, &self.p.symbols);
                        }
                        let idx = table.index.entry(*col).or_insert_with(|| {
                            let mut m = Map::<Id, Vec<u32>>::default();
                            if let Some(column) = table.cells.0.get(*col) {
                                for (r, &cell) in column.0.iter().enumerate() {
                                    let symbol = if integer {
                                        self.codes.symbol(cell)
                                    } else {
                                        self.p
                                            .symbols
                                            .get(languages::facts::text(self.words, cell).0)
                                            .copied()
                                    };
                                    if let Some(id) = symbol {
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
        mut read: impl FnMut(Relation, &mut FileWords, &mut FilledColumns) -> bool,
        alive: &mut [bool],
    ) -> u64 {
        self.ordered_file(
            &mut Scratch::default(),
            None,
            t,
            src,
            path,
            words,
            |r, _, w, o| Some(read(r, w, o)),
            alive,
        )
        .steps
    }
    pub(super) fn ordered_file(
        &self,
        scratch: &mut Scratch,
        order: Option<&Order>,
        t: &NodeTable,
        src: &[u8],
        path: Option<&[u8]>,
        words: &mut FileWords,
        mut read: impl FnMut(Relation, Request<'_>, &mut FileWords, &mut FilledColumns) -> Option<bool>,
        alive: &mut [bool],
    ) -> Work {
        languages::facts::clear_words(words);
        scratch.codes.next_file(words);
        scratch.arena.words.clear();
        scratch.sets.clear();
        scratch.sets.resize(self.domains.len(), None);
        scratch.needs.clear();
        scratch.needs.resize(self.needs.len(), None);
        scratch.facts.resize_with(self.facts.len(), Table::default);
        for table in &mut scratch.facts {
            table.read = false;
            table.complete = false;
            table.index.clear();
            table.by_node.clear();
            for col in &mut table.cells.0 {
                col.0.clear();
            }
        }
        let mut f = File {
            p: self,
            order,
            t,
            src,
            path,
            read: &mut read,
            words,
            arena: &mut scratch.arena,
            codes: &mut scratch.codes,
            counts: &mut scratch.counts,
            facts: &mut scratch.facts,
            sets: &mut scratch.sets,
            needs: &mut scratch.needs,
            texts: None,
            steps: 0,
            candidates: 0,
            node_tests: 0,
            local: Map::default(),
            edges: edge::Answers::default(),
            bucket_texts: Map::default(),
            absent: Vec::new(),
            bucket_requests: 0,
            visited_buckets: Default::default(),
        };
        f.collect_absent();
        for (r, (a, (_, root))) in alive.iter_mut().zip(&self.roots).enumerate() {
            if *a {
                if let Some(&(lo, hi)) = order.and_then(|o| o.limits.get(r)) {
                    if hi != 0 && hi != *root && !f.pass(hi) {
                        *a = false;
                        continue;
                    }
                    if lo != 1 && lo != *root && f.pass(lo) {
                        continue;
                    }
                }
                *a = f.pass(*root)
            }
        }
        scratch.peak_words = scratch.peak_words.max(f.arena.words.len());
        scratch.total_words += f.arena.words.len() as u64;
        Work {
            steps: f.steps,
            candidates: f.candidates,
            local_tests: f.node_tests,
            bucket_requests: f.bucket_requests,
            distinct_buckets: f.visited_buckets.len(),
        }
    }
}
