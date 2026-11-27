//! Per-instruction execution order and whole-family counterfactuals. The ordering
//! depends only on the declared provider access and the operation's inputs.
use super::{ANY, Domain, Id, Need, Program};

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
impl Family {
    pub(crate) fn bit(self) -> u16 {
        1 << self as u8
    }
    pub(crate) fn price(self) -> u8 {
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
        Need::CapturedPath(_) | Need::PathGlob(_) | Need::PathMatches(..) => Filename,
        Need::SourceContains(_) => Sets,
        Need::SyntaxMatches(..) => Text,
        Need::Some(x) => family(&p.domains[x], &p.facts),
        _ => Logic,
    }
}

#[derive(Default, Clone, Copy)]
pub(crate) struct Price {
    pub(crate) mask: u16,
    pub(crate) cost: u8,
}
impl Price {
    pub(crate) fn of(f: Family) -> Self {
        Self {
            mask: f.bit(),
            cost: f.price(),
        }
    }
    pub(crate) fn with(self, r: Self) -> Self {
        Self {
            mask: self.mask | r.mask,
            cost: self.cost.max(r.cost),
        }
    }
}
pub(crate) struct SchedulePlan<'a> {
    pub(crate) p: &'a mut Program,
    pub(crate) nodes: Vec<Option<Price>>,
    pub(crate) files: Vec<Option<Price>>,
    pub(crate) omit: u16,
}
impl SchedulePlan<'_> {
    pub(crate) fn file(&mut self, id: Id) -> Price {
        if let Some(z) = self.files[id] {
            return z;
        }
        let class = file_family(self.p, id);
        let mut z = Price::of(class);
        match self.p.needs[id] {
            Need::Both(a, b) | Need::Either(a, b) => {
                let (left, right) = (self.file(a), self.file(b));
                z = z.with(left).with(right);
                if left.cost > right.cost {
                    match &mut self.p.needs[id] {
                        Need::Both(x, y) | Need::Either(x, y) => std::mem::swap(x, y),
                        _ => unreachable!(),
                    }
                }
            }
            Need::Some(x) => z = z.with(self.node(x)),
            Need::SyntaxMatches(..) | Need::SourceContains(_) | Need::CapturedPath(_) | Need::PathGlob(_) | Need::PathMatches(..) if self.omit & class.bit() != 0 => {
                self.p.needs[id] = Need::Any
            }
            _ => {}
        }
        self.files[id] = Some(z);
        z
    }
    pub(crate) fn node(&mut self, id: Id) -> Price {
        if let Some(z) = self.nodes[id] {
            return z;
        }
        let class = family(&self.p.domains[id], &self.p.facts);
        let mut z = Price::of(class);
        match self.p.domains[id] {
            Domain::Both(a, b) | Domain::Either(a, b) => {
                let (left, right) = (self.node(a), self.node(b));
                z = z.with(left).with(right);
                if left.cost > right.cost {
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
                let required = slots.iter().fold(Price::default(), |price, &slot| {
                    price.with(Price::of(self.p.facts[slot].family))
                });
                let (left, diff, guard) = (self.node(a), self.node(b), self.file(g));
                z = z.with(left).with(diff).with(guard);
                z = z.with(required);
                if self.omit & (class.bit() | diff.mask | required.mask) != 0 {
                    self.p.domains[id] = Domain::Both(a, ANY);
                }
            }
            Domain::When(g, n) => z = z.with(self.file(g)).with(self.node(n)),
            Domain::Member(slot, _, project, _) => {
                z = z
                    .with(Price::of(self.p.facts[slot].family))
                    .with(Price::of(self.p.facts[project].family));
                if self.omit & z.mask != 0 {
                    self.p.domains[id] = Domain::Any;
                }
            }
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
pub(crate) fn arrange(p: &mut Program, omit: u16) {
    let mut walk = SchedulePlan {
        nodes: vec![None; p.domains.len()],
        files: vec![None; p.needs.len()],
        p,
        omit: 0,
    };
    // Visit exact outputs first. Only their independent positive guards may widen.
    let certs = std::mem::take(&mut walk.p.exact);
    for (_, c) in &certs {
        walk.file(c.guard);
        walk.node(c.nodes);
    }
    walk.omit = omit;
    for i in 0..walk.p.roots.len() {
        walk.file(walk.p.roots[i].1);
    }
    walk.p.exact = certs;
    let origins = super::compact::compact(walk.p);
    let costs: Vec<_> = origins
        .into_iter()
        .map(|i| walk.files[i].map_or(0, |v| v.cost))
        .collect();
    super::groups::compile(walk.p, &costs);
}
/// Costly downward masks and scheduled-row projections are optional only
/// on the positive pass. The proof pass always sees the declared relation.
pub(crate) fn omit_positive(d: &Domain, facts: &[super::Fact], omit: u16) -> bool {
    let class = family(d, facts);
    matches!(class, Child | Schedule | Count) && omit & class.bit() != 0
}
pub(crate) fn omitted() -> u16 {
    Child.bit() | Schedule.bit() | Count.bit()
}

