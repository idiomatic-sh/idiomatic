//! Retain only instructions reachable from a rule decision or an exact output.
use super::{ANY, Domain, EMPTY, Id, Need, Program};

// A valid slot indexes a vector, so its id is always smaller than usize::MAX.
const UNSEEN: Id = usize::MAX;

struct Compact<T> {
    old: Vec<T>,
    ids: Vec<Id>,
    kept: Vec<T>,
}
impl<T> Compact<T> {
    fn new(old: Vec<T>) -> Self {
        Self {
            ids: vec![UNSEEN; old.len()],
            old,
            kept: Vec::new(),
        }
    }
    fn keep(&mut self, old: Id, value: T) -> Id {
        let new = self.kept.len();
        self.ids[old] = new;
        self.kept.push(value);
        new
    }
}
struct Reachable<'a> {
    patterns: &'a [super::regexes::Pattern],
    nodes: Compact<Domain>,
    files: Compact<Need>,
    exist: Vec<Id>,
    file_origins: Vec<Id>,
}
impl Reachable<'_> {
    fn file(&mut self, old: Id) -> Id {
        let id = self.files.ids[old];
        if id != UNSEEN {
            return id;
        }
        let mut value = std::mem::replace(&mut self.files.old[old], Need::Any);
        match &mut value {
            Need::InvalidPattern(slot) => {
                let id = if self.patterns[*slot].compiled().is_some() {
                    EMPTY
                } else {
                    ANY
                };
                self.files.ids[old] = id;
                return id;
            }
            Need::Terms(..) => unreachable!("groups are compiled after compaction"),
            Need::Some(d) => *d = self.node(*d),
            Need::Both(a, b) | Need::Either(a, b) => {
                *a = self.file(*a);
                *b = self.file(*b);
            }
            Need::PathMatches(..) | Need::CapturedPath(_) | Need::SyntaxMatches(..) | Need::SourceContains(_) | Need::Any | Need::Empty => {}
        }
        let alias = match value {
            Need::Any if old != ANY => Some(ANY),
            Need::Empty if old != EMPTY => Some(EMPTY),
            Need::Some(d) if d <= EMPTY => Some(d),
            Need::Some(d) => {
                let prev = self.exist[d];
                if prev != UNSEEN {
                    Some(prev)
                } else {
                    self.exist[d] = self.files.kept.len();
                    None
                }
            }
            Need::Both(a, b) if a == EMPTY || b == EMPTY => Some(EMPTY),
            Need::Both(a, b) if a == ANY || a == b => Some(b),
            Need::Both(a, ANY) => Some(a),
            Need::Either(a, b) if a == ANY || b == ANY => Some(ANY),
            Need::Either(a, b) if a == EMPTY || a == b => Some(b),
            Need::Either(a, EMPTY) => Some(a),
            _ => None,
        };
        if let Some(id) = alias {
            self.files.ids[old] = id;
            return id;
        }
        self.file_origins.push(old);
        self.files.keep(old, value)
    }
    fn node(&mut self, old: Id) -> Id {
        let id = self.nodes.ids[old];
        if id != UNSEEN {
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
            Domain::CountUnder(d, ..)
            | Domain::Parents(d)
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
            | Domain::SameRow(..)
            | Domain::Fact(..)
            | Domain::Flag(..)
            | Domain::Number(..)
            | Domain::OtherValues(..)
            | Domain::RelatedIds(..)
            | Domain::Member(..)
            | Domain::Unmatched(..)
            | Domain::Matching(..)
            | Domain::CapturedSuffix(..) => {}
        }
        // Keep the distinction between a literal universe and an unknown
        // computed mask. Exact differences and distinct counts inspect it.
        let alias = match value {
            Domain::Any if old != ANY => Some(ANY),
            Domain::Empty if old != EMPTY => Some(EMPTY),
            Domain::Both(ANY, b) if b != ANY => Some(b),
            Domain::Both(a, ANY) if a != ANY => Some(a),
            Domain::Both(a, b) | Domain::Either(a, b) if a == b && a != ANY => Some(a),
            Domain::When(ANY, d) if d != ANY => Some(d),
            Domain::ProvenExcept(left, _, ANY, _) => Some(left),
            Domain::ProvenExcept(_, diff, EMPTY, ref slots) if slots.is_empty() => Some(diff),
            _ => None,
        };
        if let Some(id) = alias {
            self.nodes.ids[old] = id;
            return id;
        }
        self.nodes.keep(old, value)
    }
}
pub(super) fn compact(p: &mut Program) -> Vec<Id> {
    let mut reachable = Reachable {
        patterns: &p.regexes,
        exist: vec![UNSEEN; p.domains.len()],
        file_origins: Vec::new(),
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
    p.exact.retain(|(_, c)| c.guard != ANY);
    p.domains = reachable.nodes.kept;
    p.needs = reachable.files.kept;
    p.domains.shrink_to_fit();
    p.needs.shrink_to_fit();
    reachable.file_origins
}
