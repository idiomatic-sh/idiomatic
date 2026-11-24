//! Necessary, table-free witnesses. A source may leave the cheaper projection
//! unconstrained. Only a surviving root asks the full program for its columns.
use super::{ANY, Builder, Domain, Id, Need};

struct Early<'a, 'b> {
    b: &'a mut Builder<'b>,
    files: Vec<Option<Id>>,
    nodes: Vec<Option<Id>>,
}
impl Early<'_, '_> {
    fn file(&mut self, i: Id) -> Id {
        if let Some(v) = self.files[i] {
            return v;
        }
        let value = match self.b.out.needs[i] {
            Need::Both(a, b) => {
                let (x, y) = (self.file(a), self.file(b));
                self.b.and(x, y)
            }
            Need::Either(a, b) => {
                let (x, y) = (self.file(a), self.file(b));
                self.b.or(x, y)
            }
            Need::Some(d) => self.exists(d),
            Need::PathGlob(_) | Need::PathMatches(..) | Need::Any | Need::Empty => i,
            Need::SyntaxMatches(..) | Need::SourceContains(_) | Need::InvalidPattern(_) => ANY,
            Need::Terms(..) => unreachable!("file scheduling follows projection"),
        };
        self.files[i] = Some(value);
        value
    }
    fn exists(&mut self, i: Id) -> Id {
        // Moving along a tree relation cannot manufacture an input row.
        match self.b.out.domains[i] {
            Domain::Parents(n)
            | Domain::Children(n)
            | Domain::Ancestors(n)
            | Domain::Descendants(n) => self.exists(n),
            Domain::When(g, n) => {
                let (g, n) = (self.file(g), self.exists(n));
                self.b.and(g, n)
            }
            _ => {
                let node = self.node(i);
                if node <= super::EMPTY {
                    node
                } else {
                    self.b.need(Need::Some(node))
                }
            }
        }
    }
    fn node(&mut self, i: Id) -> Id {
        if let Some(v) = self.nodes[i] {
            return v;
        }
        let value = match self.b.out.domains[i] {
            Domain::Kind(_) | Domain::Field(_) | Domain::Empty | Domain::Any | Domain::Text(_) => i,
            Domain::When(g, d) => {
                let (g, d) = (self.file(g), self.node(d));
                if g == ANY {
                    d
                } else {
                    self.b.domain(Domain::When(g, d))
                }
            }
            Domain::Both(a, b) => {
                let (x, y) = (self.node(a), self.node(b));
                self.b.both(x, y)
            }
            Domain::Either(a, b) => {
                let (x, y) = (self.node(a), self.node(b));
                self.b.either(x, y)
            }
            Domain::ProvenExcept(a, ..) | Domain::Except(a, _) => self.node(a),
            _ => ANY,
        };
        self.nodes[i] = Some(value);
        value
    }
}
pub(super) fn schedule(b: &mut Builder<'_>) {
    let mut early = Early {
        files: vec![None; b.out.needs.len()],
        nodes: vec![None; b.out.domains.len()],
        b,
    };
    for rule in 0..early.b.out.roots.len() {
        let full = early.b.out.roots[rule].1;
        let cheap = early.file(full);
        if cheap != ANY && cheap != full {
            early.b.out.roots[rule].1 = early.b.need(Need::Both(cheap, full));
        }
    }
}
