//! Lower associative file guards to short-circuit instruction lists. The
//! provider still supplies every fact and the original program owns each test.
use super::pair_summary::{Domain, Id, Need, Program};
use rustc_hash::FxHashSet;

pub(super) struct Schedule {
    pub(super) postings: super::pair_checks::postings::Layout,
    pub(super) literals: super::summary_literals::Layout,
    pub(super) rules: super::summary_rules::Rules,
    pub(super) symbols: Vec<Option<Id>>,
    pub(super) price: Vec<u8>,
    pub(super) alias: Vec<Option<Id>>,
    pub(super) needs: Vec<Vec<Id>>,
    pub(super) cost: Vec<u8>,
}
impl Schedule {
    pub(super) fn new(
        p: &Program,
        names: Option<(&std::collections::HashMap<Vec<u8>, u16>, u16)>,
        symbols: Vec<Option<Id>>,
    ) -> Self {
        let mut cost: Vec<u8> = Vec::with_capacity(p.domains.len());
        for d in &p.domains {
            let c = match d {
                Domain::Empty
                | Domain::Any
                | Domain::Kind(_)
                | Domain::Field(_)
                | Domain::HasParent => 0,
                Domain::Text(_) => 1,
                Domain::TextMatching(..) | Domain::SpanMatching(..) => 3,
                Domain::Both(a, b) | Domain::Either(a, b) | Domain::Except(a, b) => {
                    cost[*a].max(cost[*b])
                }
                Domain::Children(x)
                | Domain::Parents(x)
                | Domain::Ancestors(x)
                | Domain::Descendants(x) => cost[*x].max(2),
                Domain::CountUnder(x, ..) => cost[*x].max(3),
                Domain::When(_, x) => cost[*x],
                Domain::ProvenExcept(a, b, ..) => cost[*a].max(cost[*b]).max(4),
                _ => 4,
            };
            cost.push(c);
        }
        let mut price = Vec::<u8>::with_capacity(p.needs.len());
        for n in &p.needs {
            price.push(match n {
                Need::InvalidPattern(_) => unreachable!("proof validation happens at load"),
                Need::Any | Need::Empty | Need::PathMatches(_) | Need::PathGlob(_) | Need::SourceContains(_) => 0,
                Need::Terms(_, children) => children.iter().map(|&id| price[id]).max().unwrap_or(0),
                Need::Some(d) => cost[*d],
                Need::Both(a, b) | Need::Either(a, b) => price[*a].max(price[*b]),
            });
        }
        let mut todo: Vec<_> = p.roots.iter().map(|r| r.1).collect();
        todo.extend(p.exact.iter().map(|r| r.1.guard));
        for d in &p.domains {
            match d {
                Domain::When(need, _) | Domain::ProvenExcept(_, _, need, _) => todo.push(*need),
                _ => {}
            }
        }
        let mut done = vec![false; p.needs.len()];
        let mut needs = vec![Vec::new(); p.needs.len()];
        let proofs = super::summary_proofs::compile(p);
        while let Some(id) = todo.pop() {
            if std::mem::replace(&mut done[id], true) {
                continue;
            }
            if let Need::Terms(_, children) = &p.needs[id] {
                todo.extend(children.iter().copied());
                continue;
            }
            let both = matches!(p.needs[id], Need::Both(..));
            if !both && !matches!(p.needs[id], Need::Either(..)) {
                continue;
            }
            let mut seen = FxHashSet::default();
            let mut stack = vec![id];
            let mut terms = Vec::new();
            while let Some(x) = stack.pop() {
                if !seen.insert(x) {
                    continue;
                }
                match p.needs[x] {
                    Need::Both(a, b) if both => stack.extend([a, b]),
                    Need::Either(a, b) if !both => stack.extend([a, b]),
                    _ => terms.push(x),
                }
            }
            if both {
                let mut redundant = FxHashSet::default();
                for n in &terms {
                    if let Need::Some(d) = p.needs[*n] {
                        redundant.extend(proofs.weaker[d].iter().copied());
                    }
                }
                terms.retain(|&n| !matches!(p.needs[n],Need::Some(d) if redundant.contains(&d)));
            }
            terms.sort_unstable_by_key(|&x| (price[x], x));
            todo.extend(terms.iter().copied());
            needs[id] = terms;
        }
        Self {
            postings: super::pair_checks::postings::Layout::new(p, &symbols),
            literals: super::summary_literals::Layout::new(p, names),
            rules: super::summary_rules::Rules::new(p, &needs, &price),
            symbols,
            alias: proofs.alias,
            needs,
            cost,
            price,
        }
    }
}
