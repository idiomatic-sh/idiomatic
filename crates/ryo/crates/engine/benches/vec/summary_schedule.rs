//! Lower associative file guards to short-circuit instruction lists. The
//! provider still supplies every fact and the original program owns each test.
use super::pair_summary::{Domain, Id, Need, Program};
use rustc_hash::FxHashSet;

pub(super) struct Schedule {
    pub(super) paths: Vec<Option<super::summary_local::Path>>,
    pub(super) needs: Vec<Vec<Id>>,
    pub(super) cost: Vec<u8>,
}

impl Schedule {
    pub(super) fn new(p: &Program) -> Self {
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
                Need::Any | Need::Empty | Need::PathMatches(_) | Need::SourceContains(_) => 0,
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
        while let Some(id) = todo.pop() {
            if std::mem::replace(&mut done[id], true) {
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
            terms.sort_unstable_by_key(|&x| (price[x], x));
            todo.extend(terms.iter().copied());
            needs[id] = terms;
        }
        let paths = if std::env::var_os("VEC_LOCAL_PROGRAM").is_some() {
            (0..p.domains.len())
                .map(|id| super::summary_local::Path::new(&p.domains, id))
                .collect()
        } else {
            Vec::new()
        };
        Self { cost, needs, paths }
    }
}
