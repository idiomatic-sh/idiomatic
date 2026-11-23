//! Per-instruction execution order and whole-family counterfactuals. The ordering
//! depends only on the declared provider access and the operation's inputs.
use super::{ANY, Domain, Id, Need, Program};
use std::cell::RefCell;

#[repr(u8)]
#[derive(Clone, Copy, Default, Eq, PartialEq, Debug)]
pub(crate) enum Family {
    #[default]
    Logic,
    Shape,
    Text,
    Fact,
    Parent,
    Child,
    Exclusion,
    Row,
    Schedule,
    Count,
    Sets,
    Filename,
    Project,
}
use Family::*;
pub(crate) const FAMILIES: [Family; 13] = [
    Logic, Shape, Text, Fact, Parent, Child, Exclusion, Row, Schedule, Count, Sets, Filename,
    Project,
];
impl Family {
    pub(crate) fn name(self) -> &'static str {
        [
            "logic", "shape", "text", "fact", "parent", "child", "exclude", "row", "schedule",
            "count", "sets", "filename", "project",
        ][self as usize]
    }
    fn bit(self) -> u16 {
        1 << self as u8
    }
    fn price(self) -> u8 {
        match self {
            Logic => 0,
            Shape => 1,
            Filename => 2,
            Project => 3,
            Text => 4,
            Parent | Child => 5,
            Exclusion => 6,
            Sets => 7,
            Count => 8,
            Fact => 9,
            Row => 10,
            Schedule => 11,
        }
    }
}
pub(crate) fn family(d: &Domain, facts: &[super::Fact]) -> Family {
    match *d {
        Domain::Member(..) | Domain::Unmatched(..) => Project,
        Domain::Any | Domain::Empty | Domain::When(..) | Domain::Both(..) | Domain::Either(..) => {
            Logic
        }
        Domain::Kind(_) | Domain::Field(_) | Domain::HasParent => Shape,
        Domain::Text(_) | Domain::TextMatching(..) | Domain::SpanMatching(..) => Text,
        Domain::Parents(_) | Domain::Ancestors(_) => Parent,
        Domain::Children(_) | Domain::Descendants(_) => Child,
        Domain::Except(..) | Domain::ProvenExcept(..) => Exclusion,
        Domain::CountUnder(..) => Count,
        Domain::SameRow(slot, _) if facts[slot].family != Schedule => Row,
        Domain::Fact(slot, _)
        | Domain::RelatedIds(slot, ..)
        | Domain::Matching(slot, ..)
        | Domain::CapturedSuffix(slot, ..)
        | Domain::Number(slot, ..)
        | Domain::Flag(slot, ..)
        | Domain::OtherValues(slot, ..)
        | Domain::SameRow(slot, _) => facts[slot].family,
    }
}
pub(crate) fn file_family(p: &Program, id: Id) -> Family {
    match p.needs[id] {
        Need::PathMatches(_) => Filename,
        Need::SourceContains(_) => Sets,
        Need::Some(x) => family(&p.domains[x], &p.facts),
        _ => Logic,
    }
}

#[derive(Default, Clone, Copy)]
struct Price {
    mask: u16,
    cost: u8,
}
impl Price {
    fn of(f: Family) -> Self {
        Self {
            mask: f.bit(),
            cost: f.price(),
        }
    }
    fn with(self, r: Self) -> Self {
        Self {
            mask: self.mask | r.mask,
            cost: self.cost.max(r.cost),
        }
    }
}
struct SchedulePlan<'a> {
    p: &'a mut Program,
    nodes: Vec<Option<Price>>,
    files: Vec<Option<Price>>,
    omit: u16,
    order: bool,
}
impl SchedulePlan<'_> {
    fn file(&mut self, id: Id) -> Price {
        if let Some(z) = self.files[id] {
            return z;
        }
        let class = file_family(self.p, id);
        let mut z = Price::of(class);
        match self.p.needs[id] {
            Need::Both(a, b) | Need::Either(a, b) => {
                let (left, right) = (self.file(a), self.file(b));
                z = z.with(left).with(right);
                if self.order && left.cost > right.cost {
                    match &mut self.p.needs[id] {
                        Need::Both(x, y) | Need::Either(x, y) => std::mem::swap(x, y),
                        _ => unreachable!(),
                    }
                }
            }
            Need::Some(x) => z = z.with(self.node(x)),
            Need::SourceContains(_) | Need::PathMatches(_) if self.omit & class.bit() != 0 => {
                self.p.needs[id] = Need::Any
            }
            _ => {}
        }
        self.files[id] = Some(z);
        z
    }
    fn node(&mut self, id: Id) -> Price {
        if let Some(z) = self.nodes[id] {
            return z;
        }
        let class = family(&self.p.domains[id], &self.p.facts);
        let mut z = Price::of(class);
        match self.p.domains[id] {
            Domain::Both(a, b) | Domain::Either(a, b) => {
                let (left, right) = (self.node(a), self.node(b));
                z = z.with(left).with(right);
                if self.order && left.cost > right.cost {
                    match &mut self.p.domains[id] {
                        Domain::Both(x, y) | Domain::Either(x, y) => std::mem::swap(x, y),
                        _ => unreachable!(),
                    }
                }
            }
            Domain::Except(a, b) => {
                let (left, right) = (self.node(a), self.node(b));
                z = z.with(left).with(right);
                if self.omit & (class.bit() | right.mask) != 0 {
                    self.p.domains[id] = Domain::Both(a, ANY);
                }
            }
            Domain::ProvenExcept(a, b, g, ref slots) => {
                let required = slots
                    .iter()
                    .fold(0, |m, &s| m | self.p.facts[s].family.bit());
                let (left, diff, guard) = (self.node(a), self.node(b), self.file(g));
                z = z.with(left).with(diff).with(guard);
                z.mask |= required;
                if self.omit & (class.bit() | diff.mask | required) != 0 {
                    self.p.domains[id] = Domain::Both(a, ANY);
                }
            }
            Domain::When(g, n) => z = z.with(self.file(g)).with(self.node(n)),
            Domain::CountUnder(n, ..) => {
                z = z.with(self.node(n));
                if self.omit & z.mask != 0 {
                    self.p.domains[id] = Domain::Any;
                }
            }
            Domain::Parents(n)
            | Domain::Children(n)
            | Domain::Ancestors(n)
            | Domain::Descendants(n) => {
                z = z.with(self.node(n));
                if self.omit & class.bit() != 0 {
                    self.p.domains[id] = Domain::Any;
                }
            }
            _ if self.omit & class.bit() != 0 => self.p.domains[id] = Domain::Any,
            _ => {}
        }
        self.nodes[id] = Some(z);
        z
    }
}
pub(super) fn arrange(p: &mut Program, order: bool, omit: u16) {
    if !order && omit == 0 {
        return;
    }
    let mut walk = SchedulePlan {
        nodes: vec![None; p.domains.len()],
        files: vec![None; p.needs.len()],
        p,
        order,
        omit,
    };
    let keep_exact = std::env::var_os("VEC_PAIR_KEEP_EXACT").is_some();
    let mut certs = std::mem::take(&mut walk.p.exact);
    if keep_exact {
        // Visit exact outputs first to preserve their shared subprograms. Only
        // the remaining necessary conditions are eligible for omission.
        walk.omit = 0;
        for (_, c) in &certs {
            walk.file(c.guard);
            walk.node(c.nodes);
        }
        walk.omit = omit;
    }
    for i in 0..walk.p.roots.len() {
        let id = walk.p.roots[i].1;
        walk.file(id);
    }
    certs.retain(|(_, c)| {
        keep_exact
            || (walk.node(c.nodes).mask
                | walk.file(c.guard).mask
                | c.facts
                    .iter()
                    .fold(0, |m, &s| m | walk.p.facts[s].family.bit()))
                & omit
                == 0
    });
    walk.p.exact = certs;
    if order {
        let costs: Vec<_> = walk.files.iter().map(|r| r.map_or(0, |v| v.cost)).collect();
        super::groups::compile(walk.p, &costs);
    }
}
pub(super) fn omitted() -> u16 {
    std::env::var("VEC_PAIR_OMIT").map_or(0, |s| {
        s.split(',').fold(0, |m, n| {
            let f = FAMILIES
                .iter()
                .copied()
                .find(|f| f.name() == n && *f != Logic)
                .expect("known test family");
            m | f.bit()
        })
    })
}
#[derive(Default, Clone, Copy)]
pub(crate) struct Sample {
    pub calls: u64,
    pub hits: u64,
    pub cross: u64,
    pub ns: f64,
}
pub(crate) struct Meter {
    pub rows: RefCell<[Sample; 13]>,
    pub loads: RefCell<Vec<u64>>,
    pub overhead: f64,
    pub fills: RefCell<Vec<(u64, u64, f64)>>,
}
impl Meter {
    pub(super) fn new(slots: usize) -> Self {
        let total = (0..1000)
            .map(|_| {
                let t = super::super::thread_cpu();
                super::super::thread_cpu() - t
            })
            .sum::<f64>();
        Self {
            rows: Default::default(),
            loads: RefCell::new(vec![0; slots]),
            fills: RefCell::new(vec![(0, 0, 0.); slots]),
            overhead: total / 1000.,
        }
    }
    pub(crate) fn report(&self, p: &Program) {
        for f in FAMILIES {
            let s = self.rows.borrow()[f as usize];
            println!("PAYSHARE\t{}\t{}", f.name(), s.cross);
            println!("PAY\t{}\t{}\t{}\t{:.0}", f.name(), s.calls, s.hits, s.ns);
        }
        for (fact, (calls, rows, seconds)) in p.facts.iter().zip(self.fills.borrow().iter()) {
            if *calls > 0 {
                println!(
                    "PAYFILL\t{}\t{}\t{calls}\t{rows}\t{seconds:.6}",
                    fact.family.name(),
                    fact.name
                );
            }
        }
        for (fact, n) in p.facts.iter().zip(self.loads.borrow().iter()) {
            if *n > 0 {
                println!("PAYLOAD\t{}\t{}\t{n}", fact.family.name(), fact.name);
            }
        }
    }
}
