//! Typed estimate of work on a route to an empty emit, in executor steps.
use super::{inputs, Domain, Id, Need, Program};
use ryo_values::domain::finish_plans::types::{FinishPlan, FinishStep, RuleEmit};

/// A union has to empty every input. A select can fail on any required input.
/// Scalar counts have one output even when their relation has none.
pub(super) fn exits(plan: &FinishPlan) -> Vec<u64> {
    let mut exit = Vec::<Option<u64>>::with_capacity(plan.steps.len());
    for step in &plan.steps {
        let sources = inputs(step);
        let source = |index: usize| sources.get(index).and_then(|s| exit[s.0]);
        let cost = match step {
            FinishStep::Base(_) => Some(1),
            FinishStep::GroupCount(g) if g.keys.is_empty() => None,
            FinishStep::Union(_) => {
                let mut total = Some(1u64);
                for (at, s) in sources.iter().enumerate() {
                    if !sources[..at].contains(s) {
                        total = total.zip(exit[s.0]).map(|(a,b)| a.saturating_add(b));
                    }
                }
                total
            }
            FinishStep::Select(_) => {
                // The select may itself remove every input row on a residual.
                Some(1 + sources.iter().filter_map(|s| exit[s.0]).min().unwrap_or(1))
            }
            _ if sources.is_empty() => Some(1),
            _ => source(0).map(|x| x.saturating_add(1)),
        };
        exit.push(cost);
    }
    plan.rules.iter().map(|r| match &r.emit {
        RuleEmit::Written(e) => exit[e.step.0].unwrap_or(1),
        RuleEmit::Unwritten => 0,
    }).collect()
}

/// An indivisible conjunction with a costly right input can still have a separate,
/// table-free projection among the root's earlier positive conjuncts.
pub(super) fn conjuncts(p: &Program, root: Id, seen: &mut [usize], generation: usize, out: &mut Vec<Id>) {
    if seen[root] == generation { return; }
    seen[root] = generation;
    match &p.needs[root] {
        Need::Both(a,b) => {
            conjuncts(p,*a,seen,generation,out);
            conjuncts(p,*b,seen,generation,out);
        }
        Need::Terms(true, terms) => {
            for &term in terms.iter() { conjuncts(p,term,seen,generation,out); }
        }
        _ => out.push(root),
    }
}

pub(super) struct Quote<'a> {
    p: &'a Program,
    files: Vec<Option<u64>>,
    domains: Vec<Option<u64>>,
}
impl<'a> Quote<'a> {
    pub(super) fn new(p: &'a Program) -> Self {
        Self { p, files: vec![None;p.needs.len()], domains: vec![None;p.domains.len()] }
    }
    pub(super) fn file(&mut self, id: Id) -> u64 {
        if let Some(ns) = self.files[id] { return ns; }
        let p = self.p;
        let ns = match &p.needs[id] {
            Need::Any | Need::Empty => 0,
            Need::PathMatches(_) | Need::SourceContains(_) => super::prices::TEXT,
            Need::Some(d) if matches!(p.domains[*d], Domain::Fact(slot,None) if p.facts[slot].key.is_some()) => super::prices::LOGIC,
            Need::Some(d) => self.node(*d),
            Need::Both(a,b) | Need::Either(a,b) => self.file(*a).max(self.file(*b)),
            Need::Terms(_,terms) => terms.iter().map(|&t| self.file(t)).max().unwrap_or(0),
            Need::InvalidPattern(_) => unreachable!("validated during compaction"),
        };
        self.files[id]=Some(ns); ns
    }
    fn node(&mut self, id: Id) -> u64 {
        if let Some(ns)=self.domains[id] { return ns; }
        use super::prices::*;
        let p=self.p;
        let read=|slot:usize| provider(p.facts[slot].family).saturating_add(PROBE);
        let ns=match &p.domains[id] {
            Domain::Any | Domain::Empty => 0,
            Domain::Kind(_) | Domain::Field(_) | Domain::HasParent => CELL,
            Domain::Text(_) | Domain::TextMatching(..) | Domain::SpanMatching(..) => TEXT,
            Domain::When(g,n) => self.file(*g).max(self.node(*n)),
            Domain::Both(a,b) | Domain::Either(a,b) | Domain::Except(a,b) => self.node(*a).max(self.node(*b)),
            Domain::ProvenExcept(a,b,g,slots) => self.node(*a).max(self.node(*b)).max(self.file(*g))
                .max(slots.iter().map(|&s| read(s)).max().unwrap_or(0)),
            Domain::Children(n)| Domain::Parents(n) | Domain::Descendants(n) | Domain::Ancestors(n)
                => self.node(*n).max(TREE),
            Domain::CountUnder(n,..) => self.node(*n).max(COUNT),
            Domain::Member(a,_,b,_) => read(*a).saturating_add(read(*b)),
            Domain::Fact(s,_) | Domain::SameRow(s,_) | Domain::Unmatched(s,..)
            | Domain::RelatedIds(s,..) | Domain::OtherValues(s,..) | Domain::Flag(s,..)
            | Domain::Number(s,..) | Domain::Matching(s,..) | Domain::CapturedSuffix(s,..) => read(*s),
        };
        self.domains[id]=Some(ns); ns
    }
}

