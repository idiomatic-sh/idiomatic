//! File predicates shared by the demand graph. Equal subexpressions have one
//! plan slot. Its per-file truth is shared by the request and execution.
use super::{Exec, File, Run};
use rustc_hash::FxHashMap;

pub(super) const ALWAYS: u32 = 0;
const NEVER: u32 = 1;

#[derive(Clone, PartialEq, Eq, Hash)]
enum Test {
    Constant(bool),
    Kind(u16),
    Field(u32),
    Seed(usize),
    HasRows(u16),
    Driver(usize),
    Symbols(u16, usize, Vec<u32>),
    File(usize),
    Combine { all: bool, terms: Vec<u32> },
}

pub(super) struct Guards {
    by_node: Vec<Option<u32>>,
    late: Vec<bool>,
    columns: FxHashMap<u16, Vec<(usize, u32)>>,
    absent: Vec<bool>,
    driver_file: Option<u32>,
    driver_values: Vec<Option<bool>>,
    // Plan steps address the prefix's compacted predicate table directly.
    driver_predicates: FxHashMap<usize, usize>,
    // Reused symbol membership marks for an already written prefix column.
    marks: Vec<u32>,
    generation: u32,
    by_test: FxHashMap<Test, u32>,
    tests: Vec<Test>,
    pub(super) circuit: super::request_circuit::Circuit,
    spellings: Option<Spellings>,
}

impl Default for Guards {
    fn default() -> Self {
        Self {
            by_node: vec![],
            late: vec![],
            columns: Default::default(), absent: vec![], driver_file: None, driver_values: vec![], driver_predicates: Default::default(), marks: vec![], generation: 0,
            by_test: FxHashMap::default(),
            tests: vec![Test::Constant(true), Test::Constant(false)],
            circuit: Default::default(), spellings: None,
        }
    }
}

impl Guards {
    fn intern(&mut self, test: Test) -> u32 {
        *self.by_test.entry(test.clone()).or_insert_with(|| {
            let id = self.tests.len() as u32;
            self.tests.push(test);
            id
        })
    }

    fn symbols(&mut self, slot: u16, col: usize, values: Vec<u32>) -> u32 {
        let id = self.intern(Test::Symbols(slot, col, values));
        let by = self.columns.entry(slot).or_default();
        if !by.contains(&(col, id)) { by.push((col, id)); }
        id
    }

    pub(super) fn register_drivers(&mut self, drivers: &[(usize, usize)]) {
        self.driver_predicates.extend(drivers.iter().copied());
    }

    pub(super) fn driver(&mut self, node: usize) -> u32 {
        match self.driver_predicates.get(&node).copied() {
            Some(predicate) => self.intern(Test::Driver(predicate)),
            None => ALWAYS,
        }
    }

    pub(super) fn clear_evidence(&mut self) {
        self.absent.fill(false);
        self.driver_file = None;
    }

    pub(super) fn combine(&mut self, all: bool, mut terms: Vec<u32>) -> u32 {
        let (identity, stop) = if all { (ALWAYS, NEVER) } else { (NEVER, ALWAYS) };
        if terms.contains(&stop) {
            return stop;
        }
        terms.retain(|id| *id != identity);
        terms.sort_unstable();
        terms.dedup();
        match terms.as_slice() {
            [] => identity,
            [one] => *one,
            _ => self.intern(Test::Combine { all, terms }),
        }
    }

    pub(super) fn compile(&mut self, projections: impl Iterator<Item = u32>) -> Vec<u32> {
        let mut literal = vec![0; self.tests.len()];
        literal[NEVER as usize] = 1;
        let mut leaves = 0;
        for (i, test) in self.tests.iter().enumerate() {
            if !matches!(test, Test::Constant(_) | Test::Combine { .. }) {
                leaves += 1;
                literal[i] = 2 * leaves as u32;
            }
        }
        let mut compiled = super::request_circuit::Compiler::new(leaves);
        for (i, test) in self.tests.iter().enumerate() {
            let Test::Combine { all, terms } = test else { continue };
            let inverse = u32::from(!all);
            let result = terms.iter().fold(0, |acc, term| compiled.and(acc, literal[*term as usize] ^ inverse));
            literal[i] = result ^ inverse;
        }
        let outputs = self.by_node.iter().flatten().copied().chain(projections);
        let map = compiled.circuit.retain_outputs(outputs.map(|guard| literal[guard as usize]));
        for value in &mut literal { *value = super::request_circuit::Circuit::remap(&map, *value); }
        for node in self.by_node.iter_mut().flatten() { *node = literal[*node as usize]; }
        // Evidence producers address only independent leaves after plan load.
        // Drop the allocated term vectors and keep the per-file inputs contiguous.
        for entries in self.columns.values_mut() {
            for (_, id) in entries { *id = (literal[*id as usize] / 2) - 1; }
        }
        self.tests.retain(|t| !matches!(t, Test::Constant(_) | Test::Combine { .. }));
        self.tests.shrink_to_fit();
        self.circuit = compiled.circuit;
        self.by_test = FxHashMap::default();
        self.driver_predicates = FxHashMap::default();
        literal
    }

    pub(super) fn begin(&mut self, nodes: usize) {
        self.by_node.resize(nodes, None);
        self.late.resize(nodes, false);
    }
}

/// Each listed symbol test can succeed only if at least one of its words occurs.
/// Overlapping substring matches keep both a word and its longer spellings.
struct Spellings {
    matcher: aho_corasick::AhoCorasick,
    groups: Vec<(usize, Vec<usize>)>,
    present: Vec<bool>,
}

impl Exec<'_> {
    pub(crate) fn prefix_decisions(&mut self, file: u32) -> &mut Vec<Option<bool>> {
        let guards = &mut self.requests.guards;
        guards.driver_file = Some(file);
        &mut guards.driver_values
    }

    pub(crate) fn prepare_spellings(&mut self, run: &Run, columns: &[(String, String, Option<char>)], identifier: fn(&str) -> bool) {
        let guards = &mut self.requests.guards;
        let mut words = Vec::new();
        let mut ids = FxHashMap::default();
        let mut groups = Vec::new();
        for (leaf, test) in guards.tests.iter().enumerate() {
            let Test::Symbols(slot, column, values) = test else { continue };
            let rel = &run.fact_rels[usize::from(*slot)].0;
            let Some((_, names, _)) = run.rels.get(rel) else { continue };
            let Some((_, _, separator)) = columns.iter().find(|(r, col, _)| r == rel && col == &names[*column]) else { continue };
            let spellings: Option<Vec<_>> = values.iter().map(|value| {
                let text = std::str::from_utf8(&run.texts[*value as usize]).ok()?;
                let word = separator.and_then(|ch| text.rsplit_once(ch)).map_or(text, |(_, last)| last);
                (word.is_ascii() && identifier(word)).then_some(word)
            }).collect();
            if let Some(spellings) = spellings.filter(|v| !v.is_empty()) {
                let tests = spellings.into_iter().map(|word| *ids.entry(word).or_insert_with(|| {
                    let id = words.len();
                    words.push(word);
                    id
                })).collect();
                groups.push((leaf, tests));
            }
        }
        if !words.is_empty() {
            let matcher = aho_corasick::AhoCorasick::builder().kind(Some(aho_corasick::AhoCorasickKind::DFA)).build(&words).expect("literal source spellings");
            guards.spellings = Some(Spellings { matcher, groups, present: vec![false; words.len()] });
        }
    }

    /// The provider certifies the supplied bytes cover the declared spellings.
    pub(crate) fn source_spellings(&mut self, source: &[u8]) {
        let g = &mut self.requests.guards;
        let Some(s) = &mut g.spellings else { return };
        s.present.fill(false);
        for found in s.matcher.find_overlapping_iter(source) {
            s.present[found.pattern().as_usize()] = true;
        }
        g.absent.resize(g.tests.len(), false);
        for (leaf, words) in &s.groups {
            g.absent[*leaf] |= !words.iter().any(|&word| s.present[word]);
        }
    }

    pub(crate) fn symbol_guards(&self, run: &Run, mut visit: impl FnMut(u32, &str, usize, &[u32])) {
        for (id, test) in self.requests.guards.tests.iter().enumerate() {
            if let Test::Symbols(slot, column, choices) = test {
                visit(id as u32, &run.fact_rels[usize::from(*slot)].0, *column, choices);
            }
        }
    }

    pub(crate) fn absent_symbols(&mut self, ids: &[u32]) {
        if ids.is_empty() { return; }
        let guards = &mut self.requests.guards;
        guards.absent.resize(guards.tests.len(), false);
        for &id in ids { guards.absent[id as usize] = true; }
    }

    pub(crate) fn prefix_symbols(&mut self, run: &Run, rel: &str, data: &[Vec<u32>], keys: Option<&[u32]>, symbols: usize, lookup: impl Fn(&[u8]) -> Option<u32>) {
        let Some(&slot) = run.fact_slots.get(rel) else { return };
        let g = &mut self.requests.guards;
        let Some(tests) = g.columns.get(&slot) else { return };
        let mut last = None;
        g.absent.resize(g.tests.len(), false);
        for &(column, id) in tests {
            let Some(values) = data.get(column).filter(|c| !c.is_empty()) else { continue };
            if last != Some(column) {
                last = Some(column);
                g.generation = g.generation.wrapping_add(1);
                if g.generation == 0 { g.marks.fill(0); g.generation = 1; }
                g.marks.resize(symbols, 0);
                let mut mark = |v: u32| { if let Some(at) = g.marks.get_mut(v as usize) { *at = g.generation; } };
                if let Some(keys) = keys { for &key in keys { mark(values[key as usize]); } }
                else { for &v in values { mark(v); } }
            }
            let Test::Symbols(_, _, choices) = &g.tests[id as usize] else { continue };
            g.absent[id as usize] |= !choices.iter().any(|v| lookup(&run.texts[*v as usize]).is_some_and(|p| g.marks.get(p as usize) == Some(&g.generation)));
        }
    }

    /// The file-only part of the same predicate used by known_empty: a
    /// select needs all sources, a union needs at least one, other operators
    /// need their declared driving input. A constant step can always run.
    pub(super) fn guard(&mut self, run: &mut Run, n: usize) -> u32 {
        if sw!("VEC_NO_KNOWN") {
            return ALWAYS;
        }
        if let Some(id) = self.requests.guards.by_node[n] {
            return id;
        }
        let need = self.need(run, n);
        let mut parts = Vec::new();
        if !need.never {
            if !sw!("VEC_NO_EMPTYPROP") && let Ok(Some(view)) = self.view(run, n)
                && let super::Native::Fact(slot) = view.base {
                parts.push(self.requests.guards.intern(Test::HasRows(slot)));
            }
            for ks in &need.kinds {
                let codes = ks.iter().map(|k| self.requests.guards.intern(Test::Kind(*k))).collect();
                parts.push(self.requests.guards.combine(false, codes));
            }
            for &s in &need.seeds {
                parts.push(self.requests.guards.intern(Test::Seed(s)));
            }
            if need.file_test {
                parts.push(self.requests.guards.intern(Test::File(n)));
            }
            if !sw!("VEC_NO_EMPTYPROP") {
                for (slot, col, values) in self.ep_literals(run, n) {
                    parts.push(self.requests.guards.symbols(slot, col, values));
                }
                for fields in self.ep_field_names(run, n) {
                    let names = fields.into_iter().map(|name| self.requests.guards.intern(Test::Field(name))).collect();
                    parts.push(self.requests.guards.combine(false, names));
                }
            }
            if !need.inputs.is_empty() {
                let inputs = need.inputs.iter().map(|i| self.guard(run, *i)).collect();
                parts.push(self.requests.guards.combine(!need.all, inputs));
            }
        }
        let id = self.requests.guards.combine(true, parts);
        self.requests.guards.late[n] = !need.never && !sw!("VEC_NO_EMPTYPROP") &&
            (self.ep_data_leaf(run, n) || need.inputs.iter().any(|i| self.requests.guards.late[*i]));
        self.requests.guards.by_node[n] = Some(id);
        id
    }

    pub(super) fn allows(&self, f: &File<'_>, run: &mut Run, guard: u32) -> bool {
        let guards = &self.requests.guards;
        guards.circuit.permits(guard, |at| match &guards.tests[at] {
            Test::Kind(k) => super::node_table::kinds_present(f.table).0.get(usize::from(*k) / 64)
                .is_some_and(|word| word.0 & (1 << (k % 64)) != 0),
            Test::Field(name) => self.ep_field_present(f, run, std::slice::from_ref(name)),
            Test::Symbols(..) => !guards.absent.get(at).copied().unwrap_or(false),
            Test::HasRows(slot) => !self.requests.proven_empty.get(usize::from(*slot)).copied().unwrap_or(false),
            Test::Driver(n) => guards.driver_file != Some(f.file) || guards.driver_values.get(*n) != Some(&Some(false)),
            Test::Seed(s) => f.seeds.get(*s).is_some_and(|rows| !rows.is_empty()),
            Test::File(n) => !self.file_fails_of(f, run, *n),
            _ => unreachable!("only independent file predicates have input slots"),
        })
    }

    /// A positive file-only predicate settles the static decision completely
    /// when neither this operator nor its inputs need an answered fact.
    pub(super) fn file_decision(&self, f: &File<'_>, run: &mut Run, n: usize) -> Option<(bool, bool)> {
        let guard = self.requests.guards.by_node.get(n).copied().flatten()?;
        Some((self.allows(f, run, guard), self.requests.guards.late[n]))
    }

    pub(super) fn pruned(&self, f: &File<'_>, run: &mut Run, n: usize) -> bool {
        self.requests.guards.by_node.get(n).copied().flatten().is_some_and(|g| !self.allows(f, run, g))
    }
}
