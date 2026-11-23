//! Prove an exclusion from a concrete syntax position before requesting its facts.
use super::*;
impl<const PROFILE: bool> File<'_, PROFILE> {
    /// An upper bound at one concrete node. No provider rows are opened. A
    /// definite miss on the excluded side suffices to keep this witness.
    fn possibly_at(&self, id: Id, at: Position) -> bool {
        if let Some(s) = self.sets.get(id) {
            return s.is_none_or(|bits| {
                self.arena.word(bits, (at.0 / 64) as usize) & (1 << (at.0 % 64)) != 0
            });
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
