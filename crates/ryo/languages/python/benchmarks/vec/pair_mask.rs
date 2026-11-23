//! Intersect syntax sets once before touching a fact-bearing conjunct. Domains
//! with no keyed fact use bit-parallel set intersections throughout.
use super::*;

#[derive(Default)]
pub(crate) struct Conjuncts {
    syntax: Vec<Id>,
    facts: Vec<Id>,
}

pub(crate) fn compile(p: &Program) -> Vec<Conjuncts> {
    let mut pure = Vec::with_capacity(p.domains.len());
    let mut plans: Vec<Conjuncts> = Vec::with_capacity(p.domains.len());
    for (i, d) in p.domains.iter().enumerate() {
        let syntactic = match *d {
            Domain::Fact(..) | Domain::Matching(..) => false,
            Domain::Both(a, b) | Domain::Either(a, b) => pure[a] && pure[b],
            Domain::Parents(x) | Domain::Children(x) | Domain::Ancestors(x) => pure[x],
            _ => true,
        };
        pure.push(syntactic);
        let mut plan = Conjuncts::default();
        if !syntactic && matches!(d, Domain::Both(..)) {
            let mut open = vec![i];
            while let Some(x) = open.pop() {
                if pure[x] {
                    plan.syntax.push(x);
                } else if let Domain::Both(a, b) = p.domains[x] {
                    open.extend([a, b]);
                } else {
                    plan.facts.push(x);
                }
            }
            plan.syntax.sort_unstable();
            plan.syntax.dedup();
            plan.facts.sort_unstable();
            plan.facts.dedup();
        }
        plans.push(plan);
    }
    plans
}

fn nonempty(set: &Nodes) -> bool {
    set.as_ref().is_none_or(|b| !b.is_empty())
}

fn intersect(arena: &mut Arena, left: Nodes, right: Nodes) -> Nodes {
    match (left, right) {
        (Some(a), Some(b)) => Some(arena.intersect(a, b)),
        (a, b) => a.or(b),
    }
}
impl File<'_> {
    pub(super) fn masked_witness(&mut self, id: Id) -> Option<bool> {
        let order = self.order?;
        let plan = &order.masks[id];
        if plan.syntax.is_empty() {
            return None;
        }
        let mut allowed = None;
        for &term in &plan.syntax {
            let nodes = self.nodes(term);
            allowed = intersect(self.arena, allowed, nodes);
            if !nonempty(&allowed) {
                return Some(false);
            }
        }
        // An unconstrained input (including a partial answer) cannot supply
        // candidate identities. Preserve the ordinary evaluator in that case.
        if allowed.is_none() {
            return None;
        }
        let mut keyed = false;
        for (i, &term) in plan.facts.iter().enumerate() {
            let point = match self.p.domains[term] {
                Domain::Fact(slot, _) | Domain::Matching(slot, _, _) => {
                    order.keys[slot].is_some() && !self.facts[slot].read
                }
                _ => false,
            };
            if point {
                keyed = true;
                continue;
            }
            let nodes = self.nodes(term);
            if !keyed && i + 1 == plan.facts.len() {
                return Some(match (&allowed, &nodes) {
                    (Some(a), Some(b)) => self.arena.intersects(*a, *b),
                    (a, b) => nonempty(a) && nonempty(b),
                });
            }
            allowed = intersect(self.arena, allowed, nodes);
            if !nonempty(&allowed) {
                return Some(false);
            }
        }
        if !keyed {
            return Some(true);
        }
        let mask = allowed.expect("bounded syntax");
        for w in 0..mask.len() {
            let mut live = self.arena.word(mask, w);
            while live != 0 {
                let node = Position(w as u32 * 64 + live.trailing_zeros());
                self.candidates += 1;
                if plan.facts.iter().all(|&term| self.contains(term, node)) {
                    return Some(true);
                }
                live &= live - 1;
            }
        }
        Some(false)
    }
}
