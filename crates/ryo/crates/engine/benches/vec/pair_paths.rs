//! Test a fixed neighbour from the surviving side of a conjunction. Fuse a
//! path only when that conjunction is its sole consumer. Other paths retain
//! their shared full mask, including paths needed by exact output.
use super::{Domain, Id, Program};
#[derive(Clone, Copy)]
pub(crate) struct Edge {
    pub(crate) candidates: Id,
    pub(crate) targets: Id,
    pub(crate) to_parent: bool,
    pub(crate) transitive: bool,
    pub(crate) existence_only: bool,
}
pub(super) fn compile(p: &Program) -> Vec<Option<Edge>> {
    let mut readers = vec![0usize; p.domains.len()];
    for d in &p.domains {
        match *d {
            Domain::Both(a, b)
            | Domain::Either(a, b)
            | Domain::Except(a, b)
            | Domain::ProvenExcept(a, b, ..) => {
                readers[a] += 1;
                readers[b] += 1;
            }
            Domain::When(_, a)
            | Domain::CountUnder(a, ..)
            | Domain::Parents(a)
            | Domain::Children(a)
            | Domain::Ancestors(a)
            | Domain::Descendants(a) => readers[a] += 1,
            _ => {}
        }
    }
    for (_, c) in &p.exact {
        readers[c.nodes] += 1;
    }
    p.domains
        .iter()
        .enumerate()
        .map(|(id, d)| {
            let Domain::Both(a, b) = *d else {
                return None;
            };
            for (candidates, path) in [(a, b), (b, a)] {
                if readers[path] != 1 {
                    continue;
                }
                let (targets, to_parent, transitive) = match p.domains[path] {
                    Domain::Children(x) => (x, true, false),
                    Domain::Descendants(x) => (x, true, true),
                    Domain::Parents(x) => (x, false, false),
                    _ => continue,
                };
                match p.domains[candidates] {
                    Domain::Kind(_)
                    | Domain::Field(_)
                    | Domain::Text(_)
                    | Domain::TextMatching(..)
                    | Domain::Fact(_, Some(_))
                    | Domain::Both(..) => {
                        return Some(Edge {
                            candidates,
                            targets,
                            to_parent,
                            transitive,
                            existence_only: readers[id] == 0,
                        });
                    }
                    _ => {}
                }
            }
            None
        })
        .collect()
}
