//! Retain only instructions reachable from a rule decision or an exact output.
use super::{ANY, Domain, EMPTY, Id, Need, Program};

struct Compact<T> {
    old: Vec<T>,
    ids: Vec<Option<Id>>,
    kept: Vec<T>,
}
impl<T> Compact<T> {
    fn new(old: Vec<T>) -> Self {
        Self {
            ids: vec![None; old.len()],
            old,
            kept: Vec::new(),
        }
    }
    fn keep(&mut self, old: Id, value: T) -> Id {
        let new = self.kept.len();
        self.ids[old] = Some(new);
        self.kept.push(value);
        new
    }
}
struct Reachable {
    nodes: Compact<Domain>,
    files: Compact<Need>,
}
impl Reachable {
    fn file(&mut self, old: Id) -> Id {
        if let Some(id) = self.files.ids[old] {
            return id;
        }
        let mut value = std::mem::replace(&mut self.files.old[old], Need::Any);
        match &mut value {
            Need::Some(d) => *d = self.node(*d),
            Need::Both(a, b) | Need::Either(a, b) => {
                *a = self.file(*a);
                *b = self.file(*b);
            }
            Need::PathMatches(_) | Need::SourceContains(_) | Need::Any | Need::Empty => {}
        }
        self.files.keep(old, value)
    }
    fn node(&mut self, old: Id) -> Id {
        if let Some(id) = self.nodes.ids[old] {
            return id;
        }
        let mut value = std::mem::replace(&mut self.nodes.old[old], Domain::Any);
        match &mut value {
            Domain::Except(a, b) | Domain::Both(a, b) | Domain::Either(a, b) => {
                *a = self.node(*a);
                *b = self.node(*b);
            }
            Domain::When(g, d) => {
                *g = self.file(*g);
                *d = self.node(*d);
            }
            Domain::ProvenExcept(left, diff, g, _) => {
                *left = self.node(*left);
                *diff = self.node(*diff);
                *g = self.file(*g);
            }
            Domain::Parents(d)
            | Domain::Children(d)
            | Domain::Ancestors(d)
            | Domain::Descendants(d) => *d = self.node(*d),
            Domain::Any
            | Domain::Empty
            | Domain::HasParent
            | Domain::Kind(_)
            | Domain::Field(_)
            | Domain::Text(_)
            | Domain::TextMatching(..)
            | Domain::SpanMatching(..)
            | Domain::Fact(..)
            | Domain::Flag(..)
            | Domain::OtherValues(..)
            | Domain::RelatedIds(..)
            | Domain::Matching(..)
            | Domain::CapturedSuffix(..) => {}
        }
        self.nodes.keep(old, value)
    }
}
pub(super) fn compact(p: &mut Program) {
    let mut reachable = Reachable {
        nodes: Compact::new(std::mem::take(&mut p.domains)),
        files: Compact::new(std::mem::take(&mut p.needs)),
    };
    for id in [ANY, EMPTY] {
        reachable.node(id);
        reachable.file(id);
    }
    for (_, root) in &mut p.roots {
        *root = reachable.file(*root);
    }
    for (_, cert) in &mut p.exact {
        cert.nodes = reachable.node(cert.nodes);
        cert.guard = reachable.file(cert.guard);
    }
    p.domains = reachable.nodes.kept;
    p.needs = reachable.files.kept;
    p.domains.shrink_to_fit();
    p.needs.shrink_to_fit();
}
