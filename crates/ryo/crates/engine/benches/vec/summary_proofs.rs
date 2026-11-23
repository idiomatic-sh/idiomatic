//! Consequences of existing tests. For node domains, stronger means subset.
//! For file guards, witnessing a tree image also witnesses its source. Neither
//! operation assumes a provider answer is complete.
use super::pair_summary::{Domain, Id, Program};
use rustc_hash::FxHashMap;

pub(super) struct Proofs {
    pub(super) weaker: Vec<Vec<Id>>,
    pub(super) alias: Vec<Option<Id>>,
}
fn consequences(p: &Program, file: bool) -> Vec<Vec<Id>> {
    let mut kinds = FxHashMap::default();
    let mut whole = FxHashMap::default();
    for (i, d) in p.domains.iter().enumerate() {
        match d {
            Domain::Kind(k) => {
                kinds.insert(*k, i);
            }
            Domain::Fact(slot, None) => {
                whole.insert(*slot, i);
            }
            _ => {}
        }
    }
    let mut result: Vec<Vec<Id>> = Vec::with_capacity(p.domains.len());
    for d in &p.domains {
        let mut terms = Vec::new();
        let mut add = |x: Id| {
            terms.push(x);
            terms.extend_from_slice(&result[x]);
        };
        match d {
            Domain::Both(a, b) => {
                add(*a);
                add(*b);
            }
            Domain::Either(a, b) => {
                for x in std::iter::once(a).chain(result[*a].iter()) {
                    if x == b || result[*b].binary_search(x).is_ok() {
                        terms.push(*x);
                    }
                }
            }
            Domain::Except(x, _) | Domain::When(_, x) => add(*x),
            Domain::ProvenExcept(x, ..) if file => add(*x),
            Domain::CountUnder(x, ..)
            | Domain::Parents(x)
            | Domain::Ancestors(x)
            | Domain::Children(x)
            | Domain::Descendants(x)
                if file =>
            {
                add(*x)
            }
            Domain::Fact(s, Some(_))
            | Domain::Matching(s, ..)
            | Domain::SameRow(s, ..)
            | Domain::CapturedSuffix(s, ..)
            | Domain::OtherValues(s, ..)
            | Domain::Flag(s, ..)
            | Domain::Number(s, ..) => terms.extend(whole.get(s)),
            Domain::TextMatching(k, _) | Domain::SpanMatching(k, _) => terms.extend(kinds.get(k)),
            _ => {}
        }
        terms.sort_unstable();
        terms.dedup();
        result.push(terms);
    }
    result
}
pub(super) fn compile(p: &Program) -> Proofs {
    let nodes = consequences(p, false);
    let alias = p
        .domains
        .iter()
        .map(|d| {
            let (a, b, both) = match *d {
                Domain::Both(a, b) => (a, b, true),
                Domain::Either(a, b) => (a, b, false),
                _ => return None,
            };
            if nodes[a].binary_search(&b).is_ok() {
                Some(if both { a } else { b })
            } else if nodes[b].binary_search(&a).is_ok() {
                Some(if both { b } else { a })
            } else {
                None
            }
        })
        .collect();
    Proofs {
        weaker: consequences(p, true),
        alias,
    }
}
