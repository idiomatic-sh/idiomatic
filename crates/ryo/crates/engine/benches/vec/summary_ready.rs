//! Reuse known file guards and node witnesses before asking for more rows.
//! A positive witness says only that there is a row. A negative witness can
//! also serve as the complete empty result for an enclosing intersection.
use super::*;
impl File<'_> {
    pub(super) fn known_witness(&self, id: Id) -> Option<bool> {
        if let Some(set) = self.sets.get(id) {
            return Some(set.is_none_or(|s| !s.is_empty()));
        }
        if let Some(known) = self.witnesses.get(id).copied().flatten() {
            return Some(known);
        }
        match self.p.domains[id] {
            Domain::Any => Some(true),
            Domain::Empty => Some(false),
            Domain::Kind(k) => {
                Some(!node_table::ids_of_kind(self.t, languages::KindBits(k)).is_empty())
            }
            Domain::Field(f) => {
                Some(node_table::field_count(self.t, languages::FieldCode(f)).0 != 0)
            }
            _ => None,
        }
    }
    pub(super) fn known_need(&self, id: Id) -> Option<bool> {
        self.needs[id].or_else(|| match self.p.needs[id] {
            Need::Some(x) => self.known_witness(x),
            Need::Any => Some(true),
            Need::Empty => Some(false),
            _ => None,
        })
    }
    pub(super) fn cached_guard(&self, id: Id, decisive: bool) -> bool {
        let Some(order) = self.order else {
            return false;
        };
        order.ready_guards
            && order.needs[id]
                .iter()
                .any(|&child| self.known_need(child) == Some(decisive))
    }
    pub(super) fn ready_first(&self, id: Id) -> u8 {
        if self.known_witness(id) == Some(false) {
            return 0;
        }
        if self.sets.get(id).is_some() {
            return 1;
        }
        match self.p.domains[id] {
            Domain::Fact(slot, _)
            | Domain::Flag(slot, ..)
            | Domain::Number(slot, ..)
            | Domain::SameRow(slot, _)
            | Domain::RelatedIds(slot, ..)
            | Domain::Matching(slot, ..)
            | Domain::OtherValues(slot, ..)
            | Domain::CapturedSuffix(slot, ..)
                if self.facts[slot].read =>
            {
                2
            }
            _ => 3,
        }
    }
}
