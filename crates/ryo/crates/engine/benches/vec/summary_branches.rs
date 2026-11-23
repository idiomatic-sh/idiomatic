//! Thread a file formula's conjunctions and disjunctions into branches.
//! Identical suffixes of different rules share the same instruction.
//! Large graphs can still refer to an original memoized file predicate.
use super::pair_summary::{Id, Need, Program};
use rustc_hash::FxHashMap;

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
pub(super) struct Branch {
    pub(super) test: Id,
    pub(super) yes: usize,
    pub(super) no: usize,
}
#[derive(Default)]
pub(super) struct Branches {
    pub(super) entry: Vec<usize>,
    pub(super) code: Vec<Branch>,
}

struct Compiler<'a> {
    p: &'a Program,
    schedule: &'a [Vec<Id>],
    compiled: FxHashMap<(Id, usize, usize), usize>,
    shared: FxHashMap<Branch, usize>,
    code: Vec<Branch>,
}
impl Compiler<'_> {
    fn atom(&mut self, test: Id, yes: usize, no: usize) -> usize {
        if yes == no {
            return yes;
        }
        let branch = Branch { test, yes, no };
        *self.shared.entry(branch).or_insert_with(|| {
            let next = self.code.len() + 2;
            self.code.push(branch);
            next
        })
    }
    fn lower(&mut self, id: Id, yes: usize, no: usize) -> usize {
        if yes == no {
            return yes;
        }
        if let Some(&x) = self.compiled.get(&(id, yes, no)) {
            return x;
        }
        let out = match self.p.needs[id] {
            Need::Any => yes,
            Need::Empty => no,
            Need::Both(..) | Need::Either(..)
                if !self.schedule[id].is_empty() && self.code.len() < self.p.needs.len() =>
            {
                let both = matches!(self.p.needs[id], Need::Both(..));
                let mut tail = if both { yes } else { no };
                let terms = &self.schedule[id];
                for &child in terms.iter().rev() {
                    tail = if both {
                        self.lower(child, tail, no)
                    } else {
                        self.lower(child, yes, tail)
                    };
                }
                tail
            }
            _ => self.atom(id, yes, no),
        };
        self.compiled.insert((id, yes, no), out);
        out
    }
}
impl Branches {
    pub(super) fn new(p: &Program, schedule: &[Vec<Id>]) -> Self {
        let mut c = Compiler {
            p,
            schedule,
            compiled: Default::default(),
            shared: Default::default(),
            code: Vec::new(),
        };
        let entry = p.roots.iter().map(|r| c.lower(r.1, 1, 0)).collect();
        Self {
            entry,
            code: c.code,
        }
    }
}
