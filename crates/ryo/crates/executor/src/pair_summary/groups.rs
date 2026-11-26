//! Fold single-use associative branches into ordered, short-circuit file clauses.
//! Shared branches keep their own file memo. Construction is linear in edges.
use super::{Domain, Need, Program};
pub fn compile(p: &mut Program, costs: &[u8]) {
    let mut readers = vec![0u32; p.needs.len()];
    for n in &p.needs {
        if let Need::Both(a, b) | Need::Either(a, b) = *n {
            readers[a] += 1;
            readers[b] += 1;
        }
    }
    for d in &p.domains {
        match *d {
            Domain::When(g, _) | Domain::ProvenExcept(_, _, g, _) => readers[g] += 1,
            _ => {}
        }
    }
    for (_, r) in &p.roots {
        readers[*r] += 1;
    }
    for (_, c) in &p.exact {
        readers[c.guard] += 1;
    }
    let mut work = Vec::new();
    let mut terms = Vec::new();
    // Parents precede their children in this reverse pass. Each single-use
    // interior is consumed by its first parent and never expanded twice.
    for root in (0..p.needs.len()).rev() {
        let all = matches!(p.needs[root], Need::Both(..));
        let (a, b) = match p.needs[root] {
            Need::Both(a, b) | Need::Either(a, b) => (a, b),
            _ => continue,
        };
        work.extend([b, a]);
        while let Some(i) = work.pop() {
            if readers[i] == 1
                && matches!(
                    (&p.needs[i], all),
                    (Need::Both(..), true) | (Need::Either(..), false)
                )
            {
                let n = std::mem::replace(&mut p.needs[i], Need::Any);
                if let Need::Both(l, r) | Need::Either(l, r) = n {
                    work.extend([r, l]);
                }
            } else {
                terms.push(i);
            }
        }
        if terms.len() > 2 {
            terms.sort_unstable_by_key(|i| (costs[*i], *i));
            terms.dedup();
            p.needs[root] = Need::Terms(all, terms.clone().into_boxed_slice());
        }
        terms.clear();
    }
}
