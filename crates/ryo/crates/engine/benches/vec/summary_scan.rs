//! Scalar portions of positive node formulas. A concrete left input makes it
//! possible to request facts after native tests have succeeded at a position.
use super::*;
#[derive(Default)]
pub(crate) struct Layout {
    info: Vec<Info>,
}
#[derive(Clone, Copy, Default)]
struct Info {
    direct: bool,
    total: bool,
    expensive: bool,
    operations: u32,
}
impl Layout {
    pub(crate) fn new(p: &Program) -> Self {
        let mut info: Vec<Info> = Vec::with_capacity(p.domains.len());
        for d in &p.domains {
            let (direct, total, expensive, operations) = match *d {
                Domain::Any => (true, false, false, 1),
                Domain::Empty | Domain::HasParent | Domain::Kind(_) | Domain::Field(_) => {
                    (true, true, false, 1)
                }
                Domain::Text(_) | Domain::TextMatching(..) | Domain::SpanMatching(..) => {
                    (true, true, true, 2)
                }
                Domain::Both(a, b) | Domain::Either(a, b) => (
                    info[a].direct || info[b].direct,
                    if matches!(d, Domain::Both(..)) {
                        info[a].total || info[b].total
                    } else {
                        info[a].total && info[b].total
                    },
                    info[a].expensive || info[b].expensive,
                    info[a].operations.saturating_add(info[b].operations),
                ),
                Domain::Children(x) | Domain::Parents(x) => (
                    info[x].total || matches!(p.domains[x], Domain::Any),
                    info[x].total || matches!(p.domains[x], Domain::Any),
                    info[x].expensive,
                    info[x].operations.saturating_add(1),
                ),
                Domain::When(_, x) => (
                    true,
                    info[x].total,
                    info[x].expensive,
                    info[x].operations.saturating_add(1),
                ),
                Domain::Fact(..) => (true, false, true, 1),
                _ => (false, false, true, 1),
            };
            info.push(Info {
                direct,
                total,
                expensive,
                operations,
            });
        }
        Self { info }
    }
}
impl File<'_> {
    fn value_at(&mut self, id: Id, at: Position) -> bool {
        if let Some(s) = self.sets.get(id) {
            return self.bit_at(s, at);
        }
        let t = self.t;
        let depth = self
            .order
            .expect("compiled node access")
            .scan
            .as_ref()
            .unwrap();
        match self.p.domains[id] {
            Domain::Empty => false,
            Domain::Any => true,
            Domain::Kind(k) => node_table::kind_of(t, at).0 == k,
            Domain::Field(f) => node_table::field_of(t, at).0 == f,
            Domain::HasParent => node_table::parent_of(t, at) != at && at.0 != 0,
            Domain::Text(ref word) => {
                self.p.text_kinds.contains(&node_table::kind_of(t, at).0)
                    && node_table::decoded_of(t, at).map_or_else(
                        || &self.src[super::super::span_bytes(t, at.0 as usize)],
                        |x| x.0.as_bytes(),
                    ) == word.as_bytes()
            }
            Domain::TextMatching(k, re) | Domain::SpanMatching(k, re) => {
                if node_table::kind_of(t, at).0 != k {
                    return false;
                }
                let span = || &self.src[super::super::span_bytes(t, at.0 as usize)];
                let bytes = if matches!(self.p.domains[id], Domain::TextMatching(..)) {
                    node_table::decoded_of(t, at).map_or_else(span, |x| x.0.as_bytes())
                } else {
                    span()
                };
                let p = self.p;
                *self
                    .byte_tests
                    .entry((re, bytes))
                    .or_insert_with(|| p.regexes[re].is_match(bytes))
            }
            Domain::Both(a, b) => {
                let (a, b) = self.first(a, b);
                self.value_at(a, at) && self.value_at(b, at)
            }
            Domain::Either(a, b) => {
                let (a, b) = self.first(a, b);
                self.value_at(a, at) || self.value_at(b, at)
            }
            Domain::When(n, x) => self.pass(n) && self.value_at(x, at),
            Domain::Children(x)
                if depth.info[x].total || matches!(self.p.domains[x], Domain::Any) =>
            {
                let parent = node_table::parent_of(t, at);
                parent != at
                    && !node_table::embedded_segments(t).any(|s| s.root == at)
                    && self.value_at(x, parent)
            }
            Domain::Parents(x)
                if depth.info[x].total || matches!(self.p.domains[x], Domain::Any) =>
            {
                if let Some(mut children) = node_table::direct_children(t, at) {
                    if children.any(|n| self.value_at(x, n)) {
                        return true;
                    }
                } else if node_table::children_of(t, at)
                    .iter()
                    .any(|&n| self.value_at(x, n))
                {
                    return true;
                }
                node_table::embedded_segments(t).any(|s| {
                    node_table::parent_of(t, s.root) == at
                        && at != s.root
                        && self.value_at(x, s.root)
                })
            }
            _ => self.point(id, at),
        }
    }
    /// Evaluate one operand on the native or already-computed node set of the
    /// other. Whole-register evaluation handles a dense input directly.
    pub(super) fn restricted(&mut self, input: Bits, id: Id, first: bool) -> Option<Bits> {
        let plan = self.order?.scan.as_ref()?;
        let i = plan.info[id];
        if !i.direct || self.sets.get(id).is_some() {
            return None;
        }
        let n = node_table::element_count(self.t).0;
        let limit = if i.expensive {
            n
        } else {
            n.div_ceil(u64::BITS as usize)
        }
        .div_ceil(i.operations as usize);
        let mut count = 0;
        for &w in &self.arena.words[input.range()] {
            count += w.count_ones() as usize;
            if count > limit {
                return None;
            }
        }
        let mut hits = Vec::new();
        for (offset, k) in input.range().enumerate() {
            let mut w = self.arena.words[k];
            while w != 0 {
                let node = Position(((input.first() + offset) as u32) * 64 + w.trailing_zeros());
                *self.visits += 1;
                if self.value_at(id, node) {
                    hits.push(node.0);
                    if first {
                        return Some(self.arena.bits(hits));
                    }
                }
                w &= w - 1;
            }
        }
        Some(self.arena.bits(hits))
    }
    pub(super) fn grouped(&mut self, id: Id, only_witness: bool) -> Option<Nodes> {
        let order = self.order?;
        let terms = order.groups.as_ref()?.nodes.get(id)?;
        if terms.is_empty() {
            return None;
        }
        let both = matches!(self.p.domains[id], Domain::Both(..));
        let mut regs = Vec::new();
        let mut last_price = 0;
        for &term in terms {
            let price = order.cost[term];
            if both && price > last_price && !regs.is_empty() && !self.arena.common(&regs) {
                return Some(Some(Bits::default()));
            }
            last_price = price;
            match self.nodes(term) {
                None if !both => return Some(None),
                Some(b) if both && b.is_empty() => return Some(Some(b)),
                Some(b) if !both && only_witness && !b.is_empty() => return Some(Some(b)),
                Some(b) if !b.is_empty() => regs.push(b),
                _ => {}
            }
        }
        if regs.is_empty() {
            return Some(if both { None } else { Some(Bits::default()) });
        }
        if both && only_witness {
            return Some(if self.arena.common(&regs) {
                None
            } else {
                Some(Bits::default())
            });
        }
        Some(Some(self.arena.group(&regs, both)))
    }
}

impl File<'_> {
    /// An upper bound at one concrete node. No provider rows are opened. A
    /// definite miss on the excluded side suffices to keep this witness.
    fn possibly_at(&self, id: Id, at: Position) -> bool {
        if let Some(s) = self.sets.get(id) {
            return self.bit_at(s, at);
        }
        match self.p.domains[id] {
            Domain::Empty => false,
            Domain::Kind(k) | Domain::TextMatching(k, _) | Domain::SpanMatching(k, _) => {
                node_table::kind_of(self.t, at).0 == k
            }
            Domain::Field(k) => node_table::field_of(self.t, at).0 == k,
            Domain::HasParent => at.0 != 0 && node_table::parent_of(self.t, at) != at,
            Domain::Text(ref word) => {
                self.p
                    .text_kinds
                    .contains(&node_table::kind_of(self.t, at).0)
                    && node_table::decoded_of(self.t, at).map_or_else(
                        || &self.src[super::super::span_bytes(self.t, at.0 as usize)],
                        |x| x.0.as_bytes(),
                    ) == word.as_bytes()
            }
            Domain::Both(a, b) => self.possibly_at(a, at) && self.possibly_at(b, at),
            Domain::Either(a, b) | Domain::ProvenExcept(a, b, ..) => {
                self.possibly_at(a, at) || self.possibly_at(b, at)
            }
            Domain::When(_, a) | Domain::Except(a, _) => self.possibly_at(a, at),
            _ => true,
        }
    }
    pub(super) fn outside(&mut self, left: Id, right: Id) -> bool {
        let set = self.nodes(left);
        if let Some(b) = set {
            for (i, &w) in self.arena.words[b.range()].iter().enumerate() {
                if w == 0 {
                    continue;
                }
                let mut w = w;
                while w != 0 {
                    let at = Position(((b.first() + i) as u32) * 64 + w.trailing_zeros());
                    if !self.possibly_at(right, at) {
                        return true;
                    }
                    w &= w - 1;
                }
                break;
            }
            false
        } else {
            (0..node_table::element_count(self.t).0.min(u64::BITS as usize))
                .any(|n| !self.possibly_at(right, Position(n as u32)))
        }
    }
}

impl<'a> File<'a> {
    fn native_ids(&self, id: Id) -> Option<&'a [Position]> {
        match self.p.domains[id] {
            Domain::Kind(k) => Some(node_table::ids_of_kind(self.t, languages::KindBits(k))),
            Domain::Field(f) => Some(node_table::ids_of_field(self.t, languages::FieldCode(f))),
            _ => None,
        }
    }
    /// A native bucket already is a sorted position list. When it supplies
    /// the candidates, borrow it directly and only store the passing nodes.
    pub(super) fn native_join(&mut self, a: Id, b: Id, first: bool) -> Option<Bits> {
        let order = self.order?;
        if !order.buckets {
            return None;
        }
        let (ids, other) = match (self.native_ids(a), self.native_ids(b)) {
            (Some(x), Some(y)) if y.len() < x.len() => (y, a),
            (Some(x), _) => (x, b),
            (_, Some(y)) => (y, a),
            _ => return None,
        };
        let mask = self.sets.get(other);
        let native = matches!(
            self.p.domains[other],
            Domain::Kind(_) | Domain::Field(_) | Domain::HasParent
        );
        if mask.is_none() && !native {
            let plan = order.scan.as_ref()?;
            let i = plan.info[other];
            if !i.direct {
                return None;
            }
            let n = node_table::element_count(self.t).0;
            let limit = if i.expensive {
                n
            } else {
                n.div_ceil(u64::BITS as usize)
            }
            .div_ceil(i.operations as usize);
            if ids.len() > limit {
                return None;
            }
        }
        let mut accepted = Vec::new();
        for &at in ids {
            *self.visits += 1;
            let ok = if let Some(s) = mask {
                self.bit_at(s, at)
            } else {
                self.value_at(other, at)
            };
            if ok {
                accepted.push(at.0);
                if first {
                    break;
                }
            }
        }
        Some(self.arena.bits(accepted))
    }
}
