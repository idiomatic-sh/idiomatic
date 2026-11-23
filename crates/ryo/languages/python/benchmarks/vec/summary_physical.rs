//! Physical form of an existing summary. Canonical intersections and unions can
//! share a register across rules even when the plan joined their inputs in a
//! different order. Pure-syntax upper/lower bounds can decide a file exactly.
use super::pair_summary::{Domain, Fact, Id, Need, Program};
use rustc_hash::FxHashMap as Map;

const ALL: Id = 0;
const EMPTY: Id = 1;

struct Builder<'a> {
    domains: Vec<Domain>,
    ds: Map<Domain, Id>,
    needs: Vec<Need>,
    ns: Map<Need, Id>,
    text_kinds: &'a [u16],
}
impl Builder<'_> {
    fn domain(&mut self, d: Domain) -> Id {
        if let Some(&id) = self.ds.get(&d) {
            return id;
        }
        let id = self.domains.len();
        self.domains.push(d.clone());
        self.ds.insert(d, id);
        id
    }
    fn need(&mut self, n: Need) -> Id {
        if let Some(&id) = self.ns.get(&n) {
            return id;
        }
        let id = self.needs.len();
        self.needs.push(n.clone());
        self.ns.insert(n, id);
        id
    }
    fn join(&mut self, both: bool, a: Id, b: Id) -> Id {
        let (unit, zero) = if both { (ALL, EMPTY) } else { (EMPTY, ALL) };
        if a == zero || b == zero {
            return zero;
        }
        if a == unit || a == b {
            return b;
        }
        if b == unit {
            return a;
        }
        let mut terms = Vec::new();
        let mut stack = vec![a, b];
        while let Some(x) = stack.pop() {
            match self.domains[x] {
                Domain::Both(a, b) if both => stack.extend([a, b]),
                Domain::Either(a, b) if !both => stack.extend([a, b]),
                _ => terms.push(x),
            }
        }
        terms.sort_unstable();
        terms.dedup();
        if both {
            let (mut kind, mut field, mut text) = (None, None, None);
            for &x in &terms {
                match self.domains[x] {
                    Domain::Kind(k) | Domain::TextMatching(k, _) => {
                        if kind.replace(k).is_some_and(|previous| previous != k) {
                            return EMPTY;
                        }
                    }
                    Domain::Field(f) => {
                        if field.replace(f).is_some_and(|previous| previous != f) {
                            return EMPTY;
                        }
                    }
                    Domain::Text(_) => {
                        if text.replace(x).is_some() {
                            return EMPTY;
                        }
                    }
                    _ => {}
                }
            }
            if text.is_some() && kind.is_some_and(|k| !self.text_kinds.contains(&k)) {
                return EMPTY;
            }
            if terms
                .iter()
                .any(|&x| matches!(self.domains[x], Domain::TextMatching(..)))
            {
                terms.retain(|&x| !matches!(self.domains[x], Domain::Kind(_)));
            }
        }
        let mut acc = terms[0];
        for &term in &terms[1..] {
            acc = self.domain(if both {
                Domain::Both(acc, term)
            } else {
                Domain::Either(acc, term)
            });
        }
        acc
    }
    fn d(&mut self, d: &Domain, from: &[Id], fact: Option<Id>) -> Id {
        match *d {
            Domain::Fact(..) | Domain::Matching(..) if fact.is_some() => fact.unwrap(),
            Domain::Both(a, b) => self.join(true, from[a], from[b]),
            Domain::Either(a, b) => self.join(false, from[a], from[b]),
            Domain::Parents(x) | Domain::Ancestors(x) | Domain::Children(x) => {
                let x = from[x];
                if x == ALL || x == EMPTY {
                    return x;
                }
                self.domain(match d {
                    Domain::Parents(_) => Domain::Parents(x),
                    Domain::Children(_) => Domain::Children(x),
                    _ => Domain::Ancestors(x),
                })
            }
            _ => self.domain(d.clone()),
        }
    }
    fn bool_join(&mut self, both: bool, a: Id, b: Id) -> Id {
        let (unit, zero) = if both { (ALL, EMPTY) } else { (EMPTY, ALL) };
        if a == zero || b == zero {
            return zero;
        }
        if a == unit || a == b {
            return b;
        }
        if b == unit {
            return a;
        }
        let (a, b) = (a.min(b), a.max(b));
        self.need(if both {
            Need::Both(a, b)
        } else {
            Need::Either(a, b)
        })
    }
    fn n(&mut self, n: &Need, ds: &[Id], ns: &[Id]) -> Id {
        match *n {
            Need::Some(d) => match ds[d] {
                ALL => ALL,
                EMPTY => EMPTY,
                d => self.need(Need::Some(d)),
            },
            Need::Both(a, b) => self.bool_join(true, ns[a], ns[b]),
            Need::Either(a, b) => self.bool_join(false, ns[a], ns[b]),
            _ => self.need(n.clone()),
        }
    }
}

pub(super) fn compile(p: &Program, bounds: bool) -> (Program, Vec<(Id, Id)>) {
    let mut b = Builder {
        domains: Vec::new(),
        ds: Map::default(),
        needs: Vec::new(),
        ns: Map::default(),
        text_kinds: &p.text_kinds,
    };
    b.domain(Domain::Any);
    b.domain(Domain::Empty);
    b.need(Need::Any);
    b.need(Need::Empty);
    let (mut exact, mut lo, mut hi) = (Vec::new(), Vec::new(), Vec::new());
    for d in &p.domains {
        exact.push(b.d(d, &exact, None));
        if bounds {
            lo.push(b.d(d, &lo, Some(EMPTY)));
            hi.push(b.d(d, &hi, Some(ALL)));
        }
    }
    let (mut needs, mut lower, mut upper) = (Vec::new(), Vec::new(), Vec::new());
    for n in &p.needs {
        needs.push(b.n(n, &exact, &needs));
        if bounds {
            lower.push(b.n(n, &lo, &lower));
            upper.push(b.n(n, &hi, &upper));
        }
    }
    let limits = if bounds {
        p.roots
            .iter()
            .map(|(_, n)| (lower[*n], upper[*n]))
            .collect()
    } else {
        Vec::new()
    };
    let out = Program {
        domains: b.domains,
        needs: b.needs,
        roots: p
            .roots
            .iter()
            .map(|(r, n)| (r.clone(), needs[*n]))
            .collect(),
        skips: p.skips.clone(),
        regexes: p.regexes.clone(),
        text_kinds: p.text_kinds.clone(),
        symbols: p.symbols.clone(),
        max_text: p.max_text,
        texts: p.texts.clone(),
        facts: p
            .facts
            .iter()
            .map(|f| Fact {
                relation: f.relation,
                key: f.key,
                kinds: f.kinds,
            })
            .collect(),
        capped: p.capped,
    };
    (out, limits)
}
