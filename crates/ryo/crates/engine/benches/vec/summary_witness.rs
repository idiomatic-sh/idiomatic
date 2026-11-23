//! A file existential does not require the full image of a tree operation.
//! Unknown and universal node sets are distinct inputs to the navigation
//! operators in the summary language; preserve the same handling as nodes().
use super::*;

impl File<'_> {
    pub(super) fn witness(&mut self, id: Id) -> bool {
        if let Some(known) = self.witnesses[id] {
            return known;
        }
        if let Some(nodes) = self.sets.get(id) {
            return nodes.is_none_or(|s| !s.is_empty());
        }
        *self.witness_calls += 1;
        if let Some(value) = self.grouped(id, true) {
            let yes = value.is_none_or(|s| !s.is_empty());
            self.witnesses[id] = Some(yes);
            return yes;
        }
        let yes = match self.p.domains[id] {
            Domain::Any => true,
            Domain::Empty => false,
            Domain::Text(_) if let Some(layout) = self.order.and_then(|o| o.literals.as_ref()) => {
                !layout.nodes(id, self.t, self.src, self.literals).is_empty()
            }
            Domain::Kind(k) => !node_table::ids_of_kind(self.t, languages::KindBits(k)).is_empty(),
            Domain::Field(k) => node_table::field_count(self.t, languages::FieldCode(k)).0 != 0,
            Domain::Either(a, b) => {
                let (a, b) = self.first(a, b);
                self.witness(a) || self.witness(b)
            }
            Domain::When(n, x)
                if self
                    .order
                    .is_some_and(|o| o.proof_order && o.cost[x] < o.price[n]) =>
            {
                self.witness(x) && self.pass(n)
            }
            Domain::Except(a, b)
                if self.order.is_some_and(|o| o.proof_order) && self.outside(a, b) =>
            {
                true
            }
            Domain::ProvenExcept(a, d, g, ref slots)
                if self.order.is_some_and(|o| o.proof_order) =>
            {
                self.witness(a) && (self.pass(g) || self.witness(d) || !self.complete(slots))
            }
            Domain::When(n, x) => self.pass(n) && self.witness(x),
            Domain::ProvenExcept(a, d, g, ref slots) => {
                self.witness(a) && (self.pass(g) || !self.complete(slots) || self.witness(d))
            }
            Domain::Both(a, b) if let Some(s) = self.native_join(a, b, true) => !s.is_empty(),
            Domain::Both(a, b) => {
                if let Some(yes) = self.local_answer(id) {
                    yes
                } else {
                    let (a, b) = self.first(a, b);
                    match self.nodes(a) {
                        Some(a) if a.is_empty() => false,
                        Some(a) if let Some(v) = self.restricted(a, b, true) => !v.is_empty(),
                        Some(a) => match self.nodes(b) {
                            Some(b) => self.arena.intersects(a, b),
                            None => true,
                        },
                        None => self.witness(b),
                    }
                }
            }
            Domain::Parents(x)
            | Domain::Ancestors(x)
            | Domain::Children(x)
            | Domain::Descendants(x) => {
                let down = matches!(
                    self.p.domains[id],
                    Domain::Children(_) | Domain::Descendants(_)
                );
                let t = self.t;
                let qualifies = |n: u32| {
                    let n = Position(n);
                    if down {
                        !node_table::children_of(t, n).is_empty()
                    } else {
                        let p = node_table::parent_of(t, n);
                        p != n && (p.0 as usize) < node_table::element_count(t).0
                    }
                };
                if matches!(self.p.domains[x], Domain::Any) {
                    matches!(self.p.domains[id], Domain::Descendants(_))
                        || (0..node_table::element_count(self.t).0 as u32).any(qualifies)
                } else {
                    self.nodes(x).is_none_or(|input| {
                        self.arena.words[input.range()]
                            .iter()
                            .enumerate()
                            .any(|(i, &word)| {
                                let mut bits = word;
                                while bits != 0 {
                                    if qualifies(
                                        (input.first() + i) as u32 * 64 + bits.trailing_zeros(),
                                    ) {
                                        return true;
                                    }
                                    bits &= bits - 1;
                                }
                                false
                            })
                    })
                }
            }
            Domain::Fact(slot, None) => {
                let spec = &self.p.facts[slot];
                let table = &mut self.facts[slot];
                if !table.read
                    && let Some(yes) = (self.read)(
                        spec.source,
                        Query::Exists,
                        self.words,
                        &mut FilledColumns::default(),
                    )
                {
                    yes
                } else {
                    table.load(
                        spec,
                        node_table::element_count(self.t).0 as u32,
                        self.words,
                        self.read,
                    );
                    !table.complete || table.cells.0.first().is_some_and(|c| !c.0.is_empty())
                }
            }
            _ => self.nodes(id).is_none_or(|s| !s.is_empty()),
        };
        self.witnesses[id] = Some(yes);
        yes
    }
}
