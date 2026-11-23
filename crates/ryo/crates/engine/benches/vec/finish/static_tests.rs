//! File-invariant premises of the emptiness proof. Records on steps hold ranges of
//! test IDs. Only a demanded test pays for its provider answer or index.
use super::{Exec, File, Native, Provider, Run};
use rustc_hash::FxHashMap;
use std::rc::Rc;

/// Words already present before binding have stable IDs. A word seen only in
/// the proof compiler stays spelled out until the live symbol table contains it.
#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum Word {
    Id(u32),
    Text(Box<[u8]>),
}

impl Word {
    pub(super) fn id(&self, run: &Run) -> Option<u32> {
        match self {
            Self::Id(id) => Some(*id),
            Self::Text(bytes) => run.ids.get(bytes.as_ref()).copied(),
        }
    }
}

#[derive(Debug, PartialEq, Eq, Hash)]
pub(super) enum Test {
    Kinds(Box<[u16]>),
    Seed(usize),
    Fields(Box<[Word]>),
    EmptyFact(u16),
    // A literal on a non-symbol column still demands the provider under the old contract.
    DemandFact(u16),
    Literal(u16, usize, Box<[Word]>),
    SmallView(usize),
}

#[derive(Default)]
pub(super) struct Tests {
    ids: FxHashMap<Rc<Test>, u32>,
    pub(super) initial_words: usize,
    unique: Vec<Rc<Test>>,
    program: Vec<u32>,
}

#[derive(Default, Clone, Copy)]
pub(super) struct List {
    start: u32,
    len: u32,
}

impl Tests {
    pub(super) fn words(&self, run: &Run, values: &[u32]) -> Box<[Word]> {
        values
            .iter()
            .map(|&v| {
                if (v as usize) < self.initial_words {
                    Word::Id(v)
                } else {
                    Word::Text(run.text(v).into())
                }
            })
            .collect()
    }

    pub(super) fn bind(&mut self, leaves: impl IntoIterator<Item = Test>) -> List {
        let start = self.program.len();
        for leaf in leaves {
            let id = if let Some(&id) = self.ids.get(&leaf) {
                id
            } else {
                let id = self.unique.len() as u32;
                let leaf = Rc::new(leaf);
                self.ids.insert(leaf.clone(), id);
                self.unique.push(leaf);
                id
            };
            self.program.push(id);
        }
        List {
            start: start as u32,
            len: (self.program.len() - start) as u32,
        }
    }
}

impl List {
    pub(super) fn is_empty(self) -> bool {
        self.len == 0
    }
}

/// Two bits per distinct test. Zero is unknown. A miss never borrows the cache
/// across a call into the provider.
#[derive(Default)]
pub(super) struct Answers(Vec<u64>);

impl Answers {
    #[inline]
    pub(super) fn get(&self, id: u32) -> Option<bool> {
        let word = *self.0.get((id / 32) as usize)?;
        match (word >> (2 * (id % 32))) & 3 {
            1 => Some(false),
            2 => Some(true),
            _ => None,
        }
    }

    pub(super) fn set(&mut self, id: u32, value: bool, tests: usize) {
        let i = (id / 32) as usize;
        if self.0.len() <= i {
            self.0.resize(tests.div_ceil(32), 0);
        }
        self.0[i] |= (1 + u64::from(value)) << (2 * (id % 32));
    }

    pub(super) fn clear(&mut self) {
        self.0.fill(0);
    }
}

impl Exec<'_> {
    pub(super) fn any_static(
        &mut self,
        f: &mut File<'_>,
        run: &mut Run,
        p: &mut dyn Provider,
        list: List,
    ) -> bool {
        for at in list.start..list.start + list.len {
            let id = self.tests.program[at as usize];
            let value = match f.static_answers.get(id) {
                Some(value) => Some(value),
                None => self.first_static(f, run, p, id),
            };
            match value {
                Some(true) => return true,
                // Only the owning view starts its fact tests with EmptyFact. A
                // failed fill leaves its literal undecided too. A select can
                // continue with another independent source after a failed fill.
                None if matches!(*self.tests.unique[id as usize], Test::EmptyFact(_)) => {
                    return false;
                }
                _ => {}
            }
        }
        false
    }

    #[inline(never)]
    fn first_static(
        &mut self,
        f: &mut File<'_>,
        run: &mut Run,
        p: &mut dyn Provider,
        id: u32,
    ) -> Option<bool> {
        use Test::*;
        // Holding the descriptor across a provider call is unnecessary, including
        // the reference count. Fact slots are stable even when a fill interns text.
        if let EmptyFact(i) | DemandFact(i) | Literal(i, ..) = *self.tests.unique[id as usize] {
            if f.facts.get(i as usize).is_none_or(Option::is_none)
                && self.load_fact(f, run, p, Native::Fact(i)).is_err()
            {
                return None;
            }
        }
        let empty = match &*self.tests.unique[id as usize] {
            Kinds(codes) => codes
                .iter()
                .all(|&c| super::node_table::ids_of_kind(f.table, super::KindBits(c)).is_empty()),
            Seed(k) => f.seeds.get(*k).is_none_or(Vec::is_empty),
            Fields(names) => !self.ep_field_present(f, run, names),
            EmptyFact(i) => f.facts[*i as usize].as_ref().is_some_and(|t| t.n == 0),
            DemandFact(_) => false,
            Literal(i, col, values) => {
                let index = f.fact_list(*i, *col);
                values
                    .iter()
                    .filter_map(|w| w.id(run))
                    .all(|v| index.get(&u64::from(v)).is_none())
            }
            SmallView(node) => self.ep_view_rows_empty(f, run, p, *node),
        };
        f.static_answers.set(id, empty, self.tests.unique.len());
        Some(empty)
    }
}
