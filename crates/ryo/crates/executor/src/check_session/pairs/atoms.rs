//! Canonical identities for proofs that depend only on a version's syntax.
//! A provider population can be filled or become partial on a later call.
use crate::pair_summary::{Domain, Id, Need, Program};
use rustc_hash::FxHashMap;

#[derive(Eq, Hash, PartialEq)]
enum Key {
    Need(Need),
    Node(Domain, bool),
    Pattern(bool, Box<str>),
}

#[derive(Default)]
pub(super) struct Atoms(FxHashMap<Key, Id>);

impl Atoms {
    fn intern(&mut self, key: Key) -> Id {
        let next = self.0.len();
        *self.0.entry(key).or_insert(next)
    }

    pub(super) fn bind(&mut self, program: &Program) -> Vec<Option<Id>> {
        let mut binding = Binding {
            atoms: self,
            program,
            needs: vec![None; program.needs.len()],
            nodes: vec![None; program.domains.len()],
        };
        for id in 0..program.needs.len() {
            binding.need(id);
        }
        binding.needs.into_iter().flatten().collect()
    }
}

struct Binding<'a> {
    atoms: &'a mut Atoms,
    program: &'a Program,
    needs: Vec<Option<Option<Id>>>,
    nodes: Vec<Option<Option<Id>>>,
}

impl Binding<'_> {
    fn pattern(&mut self, id: Id) -> Id {
        let pattern = &self.program.regexes[id];
        self.atoms
            .intern(Key::Pattern(pattern.is_glob(), pattern.as_str().into()))
    }

    fn need(&mut self, id: Id) -> Option<Id> {
        if let Some(value) = self.needs[id] {
            return value;
        }
        let result = self.need_key(id).map(|key| self.atoms.intern(key));
        self.needs[id] = Some(result);
        result
    }

    fn need_key(&mut self, id: Id) -> Option<Key> {
        let key = match &self.program.needs[id] {
            Need::Any => Need::Any,
            Need::Empty => Need::Empty,
            Need::PathMatches(re, yes) => Need::PathMatches(self.pattern(*re), *yes),
            Need::SyntaxMatches(re, yes) => Need::SyntaxMatches(self.pattern(*re), *yes),
            Need::SourceContains(re) => Need::SourceContains(self.pattern(*re)),
            Need::CapturedPath(capture) => Need::CapturedPath(capture.clone()),
            Need::PathGlob(glob) => {
                let pattern = self.program.path_globs[*glob].as_str();
                Need::PathGlob(self.atoms.intern(Key::Pattern(true, pattern.into())))
            }
            Need::Some(node) => Need::Some(self.node(*node)?),
            Need::Both(a, b) => Need::Both(self.need(*a)?, self.need(*b)?),
            Need::Either(a, b) => Need::Either(self.need(*a)?, self.need(*b)?),
            Need::Terms(all, terms) => {
                let mut canonical = terms
                    .iter()
                    .map(|&child| self.need(child))
                    .collect::<Option<Vec<_>>>()?;
                canonical.sort_unstable();
                Need::Terms(*all, canonical.into_boxed_slice())
            }
            Need::InvalidPattern(_) => return None,
        };
        Some(Key::Need(key))
    }

    fn node(&mut self, id: Id) -> Option<Id> {
        if let Some(value) = self.nodes[id] {
            return value;
        }
        let value = self.node_key(id).map(|key| self.atoms.intern(key));
        self.nodes[id] = Some(value);
        value
    }

    fn node_key(&mut self, id: Id) -> Option<Key> {
        let node = match &self.program.domains[id] {
            value @ (Domain::Any
            | Domain::Empty
            | Domain::HasParent
            | Domain::Kind(_)
            | Domain::Field(_)
            | Domain::Text(_)) => value.clone(),
            Domain::TextMatching(kind, regex) => Domain::TextMatching(*kind, self.pattern(*regex)),
            Domain::SpanMatching(kind, regex) => Domain::SpanMatching(*kind, self.pattern(*regex)),
            Domain::CountUnder(root, depth, cmp, n) => {
                Domain::CountUnder(self.node(*root)?, *depth, *cmp, *n)
            }
            Domain::When(guard, root) => Domain::When(self.need(*guard)?, self.node(*root)?),
            Domain::Both(a, b) => Domain::Both(self.node(*a)?, self.node(*b)?),
            Domain::Either(a, b) => Domain::Either(self.node(*a)?, self.node(*b)?),
            Domain::Except(a, b) => Domain::Except(self.node(*a)?, self.node(*b)?),
            Domain::Parents(root) => Domain::Parents(self.node(*root)?),
            Domain::Children(root) => Domain::Children(self.node(*root)?),
            Domain::Ancestors(root) => Domain::Ancestors(self.node(*root)?),
            Domain::Descendants(root) => Domain::Descendants(self.node(*root)?),
            _ => return None,
        };
        Some(Key::Node(node, self.program.file_only[id]))
    }
}
