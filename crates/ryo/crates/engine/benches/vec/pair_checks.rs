//! Evaluate each requested witness once per file. Unknown or partial provider answers admit the file.
use super::pair_summary::{Domain, Id, Need, Program, Source};
use rustc_hash::FxHashMap as Map;
use ryo::languages::{self, FileWords, FilledColumns, NodeTable, Position, node_table};
use std::rc::Rc;

#[path = "pair_counts.rs"]
mod counts;

type Bits = Rc<Vec<u64>>;
// None is the unconstrained node set. An empty vector is the empty set.
type Nodes = Option<Bits>;
type Reader<'a> = dyn FnMut(Source, &mut FileWords, &mut FilledColumns) -> bool + 'a;

#[derive(Default)]
struct SymbolRows {
    rows: Map<Id, Vec<u32>>,
    all_words_resolved: bool,
    all_values_indexed: bool,
}

#[derive(Default)]
struct Table {
    read: bool,
    answered: bool,
    complete: bool,
    cells: FilledColumns,
    index: Map<usize, SymbolRows>,
}
struct File<'a> {
    p: &'a Program,
    file: languages::FileId,
    t: &'a NodeTable,
    src: &'a [u8],
    path: Option<&'a [u8]>,
    read: &'a mut Reader<'a>,
    words: &'a mut FileWords,
    facts: Vec<Table>,
    sets: Vec<Option<Nodes>>,
    needs: Vec<Option<bool>>,
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
        file: languages::FileId,
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
                    .is_some_and(|c| c.0.iter().all(|x| {
                        x.0 >> 32 == (match spec.source { super::pair_summary::Source::Catalog(_) => u64::from(file.0), super::pair_summary::Source::Walk(_) => 0 }) && (x.0 as u32) < n
                    }));
        }
    }
}

impl File<'_> {
    fn complete(&mut self, slots: &[usize]) -> bool {
        let n = node_table::element_count(self.t).0 as u32;
        slots.iter().all(|&slot| {
            let table = &mut self.facts[slot];
            table.load(&self.p.facts[slot], n, self.file, self.words, self.read);
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
                Domain::Fact(slot, None) => {
                    let table = &mut self.facts[*slot];
                    table.load(
                        &self.p.facts[*slot],
                        node_table::element_count(self.t).0 as u32,
                        self.file,
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
        if let Some(s) = &self.sets[id] {
            return s.clone();
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
                Some(bits(
                    node_table::ids_of_kind(self.t, languages::KindBits(*k))
                        .iter()
                        .filter_map(|p| {
                            let text = node_table::decoded_of(self.t, *p).map_or_else(
                                || &self.src[super::span_bytes(self.t, p.0 as usize)],
                                |t| t.0.as_bytes(),
                            );
                            regex.is_match(text).then_some(p.0)
                        }),
                ))
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
            Domain::Either(a, b) => match (self.nodes(*a), self.nodes(*b)) {
                (Some(a), Some(b)) => {
                    let (a, b) = if a.len() >= b.len() { (a, b) } else { (b, a) };
                    let mut v = (*a).clone();
                    for (x, y) in v.iter_mut().zip(b.iter()) {
                        *x |= *y
                    }
                    Some(Rc::new(v))
                }
                _ => None,
            },
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
                table.load(&self.p.facts[*slot], n, self.file, self.words, self.read);
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
                table.load(spec, n, self.file, self.words, self.read);
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
                table.load(spec, n, self.file, self.words, self.read);
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
                table.load(spec, n, self.file, self.words, self.read);
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
                table.load(spec, n, self.file, self.words, self.read);
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
            Domain::Matching(slot, col, re) => {
                let spec = &self.p.facts[*slot];
                let table = &mut self.facts[*slot];
                table.load(spec, n, self.file, self.words, self.read);
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
                            .filter_map(|(r, cell)| {
                                let yes = if spec.kinds[*col] == languages::CellKind::Span {
                                    let span = (cell.0 >> 32) as usize..(cell.0 as u32) as usize;
                                    self.src.get(span).is_some_and(|text| regex.is_match(text))
                                } else {
                                    regex.is_match(
                                        languages::facts::text(self.words, *cell).0.as_bytes(),
                                    )
                                };
                                yes.then(|| key.map_or(0, |c| c.0[r].0 as u32))
                                    .filter(|x| spec.key.is_none() || *x < n)
                            }),
                    ))
                }
            }
            Domain::Fact(slot, test) => {
                let spec = &self.p.facts[*slot];
                let table = &mut self.facts[*slot];
                table.load(spec, n, self.file, self.words, self.read);
                if !table.complete {
                    None
                } else {
                    let key = spec.key.and_then(|k| table.cells.0.get(k));
                    let node = |r: usize| key.map_or(0, |c| c.0[r].0 as u32);
                    if let Some((col, values)) = test {
                        let idx = table.index.entry(*col).or_insert_with(|| {
                            let mut index = SymbolRows::default();
                            if let Some(column) = table.cells.0.get(*col) {
                                let end = languages::facts::words_end(self.words).0;
                                index.all_words_resolved = true;
                                index.all_values_indexed = true;
                                for (r, &cell) in column.0.iter().enumerate() {
                                    // Out-of-dictionary cells may have a different
                                    // catalog encoding. Their spelling is unknown here.
                                    index.all_words_resolved &= cell.0 < end;
                                    let text = languages::facts::text(self.words, cell);
                                    if let Some(&id) = self.p.symbols.get(text.0) {
                                        index.rows.entry(id).or_default().push(node(r));
                                    } else {
                                        index.all_values_indexed = false;
                                    }
                                }
                            }
                            index
                        });
                        Some(bits(
                            values.iter().filter_map(|v| idx.rows.get(v)).flatten().copied(),
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

/// Completed facts and already-decided conditional inputs from the file witness pass.
pub(super) trait WitnessResults {
    fn table(&mut self, rel: languages::Relation, cells: FilledColumns);
    fn empty_driver(&mut self, step: usize);
    fn indexed_symbols(&mut self, relation: &str, column: usize, present: &dyn Fn(&str) -> Option<bool>);
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
        file: languages::FileId,
        completed: Option<&mut dyn WitnessResults>,
        mut exact: Option<&mut Vec<(usize, Bits)>>,
    ) -> u64 {
        languages::facts::clear_words(words);
        let mut f = File {
            p: self,
            file,
            t,
            src,
            path,
            read: &mut read,
            words,
            facts: (0..self.facts.len()).map(|_| Table::default()).collect(),
            sets: vec![None; self.domains.len()],
            needs: vec![None; self.needs.len()],
            texts: None,
            steps: 0,
        };
        for (a, (_, root)) in alive.iter_mut().zip(&self.roots) {
            if *a {
                *a = f.pass(*root)
            }
        }
        if let Some(out) = exact.as_mut() {
            for (rule, cert) in &self.exact {
                let rule = *rule;
                if alive[rule] && !f.pass(cert.guard) && f.complete(&cert.facts) {
                    let nodes = f
                        .nodes(cert.nodes)
                        .expect("certified domains contain only total node tests");
                    out.push((rule, nodes));
                }
            }
        }
        if let Some(completed) = completed {
            // Only publish a prerequisite the prefix already evaluated. Leave
            // the work and the invocation counts of the driving step intact.
            for &(step, need) in &self.drivers {
                if f.needs[need] == Some(false) {
                    completed.empty_driver(step);
                }
            }
            for (fact, table) in self.facts.iter().zip(f.facts) {
                if table.answered && let super::pair_summary::Source::Catalog(relation) = fact.source {
                    for (column, index) in table.index {
                        if index.all_words_resolved {
                            completed.indexed_symbols(fact.name, column, &|text| {
                                match self.symbols.get(text) {
                                    Some(id) => Some(index.rows.contains_key(id)),
                                    None if index.all_values_indexed => Some(false),
                                    None => None,
                                }
                            });
                        }
                    }
                    completed.table(relation, table.cells);
                }
            }
        }
        f.steps
    }
}
