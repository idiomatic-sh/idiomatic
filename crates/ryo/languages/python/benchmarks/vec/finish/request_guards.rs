//! File predicates shared by the demand graph. Equal subexpressions have one
//! plan slot, evaluated on first use in a file, before any provider facts exist.
use super::{Exec, File, Run};
use rustc_hash::FxHashMap;

pub(super) const ALWAYS: u32 = 0;
const NEVER: u32 = 1;

#[derive(Clone, PartialEq, Eq, Hash)]
enum Test {
    Constant(bool),
    Kinds(Vec<u16>),
    Fields(Vec<u16>),
    Seed(usize),
    File(usize),
    Combine { all: bool, terms: Vec<u32> },
}

pub(super) struct Guards {
    by_node: Vec<Option<u32>>,
    late: Vec<bool>,
    by_test: FxHashMap<Test, u32>,
    tests: Vec<Test>,
    values: Vec<Option<bool>>,
}

impl Default for Guards {
    fn default() -> Self {
        Self {
            by_node: vec![],
            late: vec![],
            by_test: FxHashMap::default(),
            tests: vec![Test::Constant(true), Test::Constant(false)],
            values: vec![],
        }
    }
}

impl Guards {
    fn intern(&mut self, test: Test) -> u32 {
        *self.by_test.entry(test.clone()).or_insert_with(|| {
            let id = self.tests.len() as u32;
            self.tests.push(test);
            self.values.push(None);
            id
        })
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

    pub(super) fn begin(&mut self, nodes: usize) {
        self.by_node.resize(nodes, None);
        self.late.resize(nodes, false);
        self.values.resize(self.tests.len(), None);
        self.values.fill(None);
    }
}

impl Exec<'_> {
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
            for ks in &need.kinds {
                parts.push(self.requests.guards.intern(Test::Kinds(ks.to_vec())));
            }
            for &s in &need.seeds {
                parts.push(self.requests.guards.intern(Test::Seed(s)));
            }
            if need.file_test {
                parts.push(self.requests.guards.intern(Test::File(n)));
            }
            if !sw!("VEC_NO_EMPTYPROP") {
                for fields in self.ep_field_codes(run, n) {
                    parts.push(self.requests.guards.intern(Test::Fields(fields)));
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

    pub(super) fn permits(&mut self, run: &mut Run, f: &File<'_>, id: u32) -> bool {
        if id == ALWAYS {
            return true;
        }
        let at = id as usize;
        if let Some(value) = self.requests.guards.values[at] {
            return value;
        }
        let value = match &self.requests.guards.tests[at] {
            Test::Constant(v) => *v,
            Test::Kinds(ks) => {
                let present = &super::node_table::kinds_present(f.table).0;
                ks.iter().any(|k| present.get(usize::from(*k) / 64).is_some_and(|w| w.0 & (1 << (k % 64)) != 0))
            }
            Test::Fields(fs) => fs
                .iter()
                .any(|c| !super::node_table::ids_of_field(f.table, ryo::languages::FieldCode(*c)).is_empty()),
            Test::Seed(s) => f.seeds.get(*s).is_some_and(|rows| !rows.is_empty()),
            Test::File(n) => {
                let n = *n;
                !self.file_fails_of(f, run, n)
            }
            Test::Combine { all, terms } => {
                let (all, len) = (*all, terms.len());
                let mut value = all;
                for i in 0..len {
                    let Test::Combine { terms, .. } = &self.requests.guards.tests[at] else {
                        unreachable!()
                    };
                    let child = terms[i];
                    if self.permits(run, f, child) != all {
                        value = !all;
                        break;
                    }
                }
                value
            }
        };
        self.requests.guards.values[at] = Some(value);
        value
    }

    /// A positive file-only predicate settles the static decision completely
    /// when neither this operator nor its inputs need an answered fact.
    pub(super) fn file_decision(&mut self, run: &mut Run, f: &File<'_>, n: usize) -> Option<(bool, bool)> {
        let guard = self.requests.guards.by_node.get(n).copied().flatten()?;
        Some((self.permits(run, f, guard), self.requests.guards.late[n]))
    }

    pub(super) fn pruned(&mut self, run: &mut Run, f: &File<'_>, n: usize) -> bool {
        self.requests.guards.by_node.get(n).copied().flatten().is_some_and(|g| !self.permits(run, f, g))
    }
}
