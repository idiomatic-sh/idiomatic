//! A file existential does not require the full image of a tree operation.
//! Unknown and universal node sets are distinct inputs to the navigation
//! operators in the summary language; preserve the same handling as nodes().
use super::*;

impl<const PROFILE: bool> File<'_, PROFILE> {
    pub(super) fn witness(&mut self, id: Id) -> bool {
        if let Some(known) = self.witnesses[id] {
            return known;
        }
        if let Some(nodes) = self.sets.get(id) {
            return nodes.is_none_or(|s| !s.is_empty());
        }
        let yes = match self.p.domains[id] {
            _ if self.plain_tree && self.p.paths[id].is_some_and(|edge| edge.existence_only) => {
                let edge = self.p.paths[id].unwrap();
                self.edge(edge, None).unwrap_or_else(|| self.nodes(id).is_none_or(|set| !set.is_empty()))
            }
            _ if self.plain_tree && self.p.paths[id].is_some() => self.nodes(id).is_none_or(|set| !set.is_empty()),
            Domain::Parents(a) | Domain::Ancestors(a) if self.plain_tree && self.p.file_only[id] => self.path_exists(a, false),
            Domain::Children(a) | Domain::Descendants(a) if self.plain_tree && self.p.file_only[id] => self.path_exists(a, true),
            Domain::Any => true,
            Domain::Empty => false,
            Domain::Text(_) => !self
                .order
                .literals
                .nodes(id, self.t, self.src, self.literals)
                .is_empty(),
            Domain::Kind(k) => !node_table::ids_of_kind(self.t, languages::KindBits(k)).is_empty(),
            Domain::Field(k) => node_table::field_count(self.t, languages::FieldCode(k)).0 != 0,
            // A bare file guard may conservatively admit an unopened keyed relation.
            // The operands of a node union instead need their projected node sets.
            Domain::Either(a, b) => {
                self.nodes(a).is_none_or(|nodes| !nodes.is_empty())
                    || self.nodes(b).is_none_or(|nodes| !nodes.is_empty())
            }
            Domain::When(n, x) if self.order.cost[x] < self.order.price[n] => {
                self.witness(x) && self.pass(n)
            }
            Domain::Except(a, b) if self.outside(a, b) => true,
            Domain::ProvenExcept(a, d, g, ref slots) => {
                self.witness(a) && (self.pass(g) || self.witness(d) || !self.complete(slots))
            }
            Domain::When(n, x) => self.pass(n) && self.witness(x),
            Domain::Both(a, b) => {
                let (a, b) = self.first(a, b);
                match self.nodes(a) {
                    Some(a) if a.is_empty() => false,
                    Some(a) => self.nodes(b).is_none_or(|b| self.arena.intersects(a, b)),
                    None => self.witness(b),
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
                        node_table::direct_children(t, n).map_or_else(
                            || !node_table::children_of(t, n).is_empty(),
                            |mut it| it.next().is_some(),
                        )
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
            Domain::Fact(_, Some(_)) => self.posted(id, true).is_none_or(|s| !s.is_empty()),
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
                } else if !table.read && spec.key.is_some() {
                    true
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
