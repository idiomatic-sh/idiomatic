//! The types of the `sources` part: the loop's frame as what answers a need's leaves.

use crate::domain::code::constants::NO_SLOT;
use crate::domain::code::types::{ConstantNumber, NeedSite, RelationNumber, Short, WordSlot};
use crate::domain::executing::measuring::service::{
    need_fill_begun, need_fill_ended, need_leaf, need_nested,
};
use crate::domain::executing::measuring::sites::service::{kind_named, row_named};
use crate::domain::executing::presence::service::node_row;
use crate::domain::executing::types::Asker;
use crate::domain::indexes::reading::service::cell_at;
use crate::domain::made_tables::reading::service::view_of;
use crate::domain::made_tables::sources::service::{made, scanned, table_of};
use crate::domain::made_tables::types::ReadSource;
use crate::domain::needs::masks::service::{clauses_back, clauses_taken};
use crate::domain::needs::masks::types::{FreeTerm, RecordCost, TermNumber};
use crate::domain::needs::types::{NeedFacts, NeedVerdict};
use crate::domain::programs::frames::types::StepFrame;
use crate::domain::programs::operands::service::constant;
use crate::domain::steps::types::MadeTableId;
use ryo_language::domain::node_table::service::kinds_present;
use ryo_relations::domain::relations::types::{Count, KindBits};

/// The loop's frame, as what a need's formula asks: its decisions are kept in the loop's state; a kind's presence is the
/// node table's; a relation's row is asked of the fill side, which fills the relation; a table's row makes the table when
/// its own need holds.
pub(crate) struct FileNeeds<'a, 's, 'f>(pub(crate) &'a mut StepFrame<'s, 'f>);

impl NeedFacts for FileNeeds<'_, '_, '_> {
    fn kept(&self, site: NeedSite) -> Option<NeedVerdict> {
        self.0
            .state
            .needs
            .get(usize::try_from(site.0).unwrap_or(usize::MAX))
            .copied()
            .flatten()
    }

    fn keep(&mut self, site: NeedSite, verdict: NeedVerdict) {
        let place = usize::try_from(site.0).unwrap_or(usize::MAX);
        if self.0.state.needs.len() <= place {
            self.0.state.needs.resize(place.saturating_add(1), None);
        }
        if let Some(slot) = self.0.state.needs.get_mut(place) {
            *slot = Some(verdict);
        }
    }

    fn kind_present(&mut self, kind: KindBits) -> NeedVerdict {
        let present = kinds_present(self.0.fill.nodes)
            .0
            .get(usize::from(kind.0 / 64))
            .is_some_and(|word| (word.0 >> (kind.0 % 64)) & 1 == 1);
        let verdict = if present {
            NeedVerdict::Holds
        } else {
            NeedVerdict::Empty
        };
        if self.0.state.measure.is_some() {
            let named = kind_named(self.0, kind);
            need_leaf(self.0.state, || format!("{named}={verdict:?}"));
        }
        verdict
    }

    fn relation_row(
        &mut self,
        relation: RelationNumber,
        column: WordSlot,
        value: WordSlot,
    ) -> NeedVerdict {
        if self.0.state.measure.is_none() {
            return self.relation_decided(relation, column, value);
        }
        // SPIKE: a measuring worker traces the term: whether it cost nothing (a node relation's sets) or a fill.
        let free = node_row(self.0, relation, column, value).is_some();
        need_nested(self.0.state, NeedVerdict::Holds);
        let verdict = self.relation_decided(relation, column, value);
        need_nested(self.0.state, NeedVerdict::Empty);
        let named = row_named(self.0, relation, column, value);
        let cost = if free { "free" } else { "fill" };
        need_leaf(self.0.state, || format!("{cost}-{named}={verdict:?}"));
        verdict
    }

    fn free_terms(&mut self, site: NeedSite) -> NeedVerdict {
        let code = self.0.fill.programs.code;
        let clauses = clauses_taken(code, site, &mut self.0.state.need_masks);
        // Every clause needs one of its terms in the file: the first clause that has none decides.
        let mut verdict = NeedVerdict::Holds;
        for clause in &clauses {
            let mut some = false;
            for term in clause {
                if self.term_holds(*term) == NeedVerdict::Holds {
                    some = true;
                    break;
                }
            }
            if !some {
                verdict = NeedVerdict::Empty;
                break;
            }
        }
        clauses_back(site, clauses, &mut self.0.state.need_masks);
        if verdict == NeedVerdict::Empty && self.0.state.measure.is_some() {
            need_leaf(self.0.state, || "free-terms=Empty".to_owned());
        }
        verdict
    }

    fn record_cost(&self, at: WordSlot) -> Option<NeedVerdict> {
        let costs = &self.0.state.need_masks.costs;
        match costs.get(usize::try_from(at.0).unwrap_or(usize::MAX)) {
            Some(RecordCost::Free) => Some(NeedVerdict::Empty),
            Some(RecordCost::Dear) => Some(NeedVerdict::Holds),
            Some(RecordCost::Unknown) | None => None,
        }
    }

    fn table_row(&mut self, table: MadeTableId) -> NeedVerdict {
        if self.0.state.measure.is_none() {
            return self.table_decided(table);
        }
        // SPIKE: a measuring worker traces the term; what is decided inside it (the table's own need, the needs its
        // fill's reads ask) is not the traced formula's.
        need_nested(self.0.state, NeedVerdict::Holds);
        let verdict = self.table_decided(table);
        need_nested(self.0.state, NeedVerdict::Empty);
        need_leaf(self.0.state, || format!("table:{}={verdict:?}", table.0));
        verdict
    }
}

impl FileNeeds<'_, '_, '_> {
    /// Whether a free term of the rule set's needs holds in the file: asked of the node table's sets the first time
    /// in a file, read after.
    fn term_holds(&mut self, term: TermNumber) -> NeedVerdict {
        const ASKED: u8 = 1;
        const HOLDS: u8 = 2;
        let frame = &mut *self.0;
        let place = usize::try_from(term.0).unwrap_or(usize::MAX);
        let stamp = Count(usize::try_from(frame.fill.store.stamp.0).unwrap_or(usize::MAX));
        let answers = &mut frame.state.need_terms;
        if answers.file != stamp {
            answers.file = stamp;
            answers.bits.clear();
        }
        if answers.bits.len() <= place {
            answers.bits.resize(
                frame
                    .state
                    .need_masks
                    .terms
                    .len()
                    .max(place.saturating_add(1)),
                0,
            );
        }
        let held = frame.state.need_terms.bits.get(place).copied().unwrap_or(0);
        if held & ASKED != 0 {
            return if held & HOLDS != 0 {
                NeedVerdict::Holds
            } else {
                NeedVerdict::Empty
            };
        }
        let verdict = match frame.state.need_masks.terms.get(place).copied() {
            Some(FreeTerm::Kinds(list)) => {
                let code = frame.fill.programs.code;
                let from = usize::try_from(list.0).unwrap_or(usize::MAX);
                let length = code
                    .pool
                    .get(from)
                    .map_or(0, |held| usize::try_from(held.0).unwrap_or(usize::MAX));
                let kinds = code
                    .pool
                    .get(from.saturating_add(1)..from.saturating_add(1).saturating_add(length))
                    .unwrap_or_default();
                let present = kinds_present(frame.fill.nodes);
                let any = kinds.iter().any(|kind| {
                    present
                        .0
                        .get(usize::try_from(kind.0 / 64).unwrap_or(usize::MAX))
                        .is_some_and(|word| (word.0 >> (kind.0 % 64)) & 1 == 1)
                });
                if any {
                    NeedVerdict::Holds
                } else {
                    NeedVerdict::Empty
                }
            }
            Some(FreeTerm::NodeRow {
                relation,
                column,
                constant,
            }) => node_row(frame, relation, column, constant).unwrap_or(NeedVerdict::Holds),
            None => NeedVerdict::Holds,
        };
        if let Some(slot) = frame.state.need_terms.bits.get_mut(place) {
            *slot = if verdict == NeedVerdict::Holds {
                ASKED | HOLDS
            } else {
                ASKED
            };
        }
        verdict
    }

    /// Whether the relation has a row in the file, with the constant in the column when both are given.
    fn relation_decided(
        &mut self,
        relation: RelationNumber,
        column: WordSlot,
        value: WordSlot,
    ) -> NeedVerdict {
        let frame = &mut *self.0;
        if let Some(decided) = node_row(frame, relation, column, value) {
            return decided;
        }
        let source = ReadSource::Filled(relation);
        let asked = scanned(
            frame.fill,
            &mut Asker {
                state: &mut *frame.state,
                found: &mut *frame.found,
                ports: frame.ports,
            },
            source,
        );
        // A relation the fill side cannot answer for says nothing: the unit is evaluated, and its own read faults.
        let Ok(whole) = asked else {
            return NeedVerdict::Holds;
        };
        let mut rows = whole.rows.start.0..whole.rows.end.0;
        let holds = if column == NO_SLOT || value == NO_SLOT {
            !rows.is_empty()
        } else {
            let wanted = constant(frame, ConstantNumber(value.0));
            let place = Short(u16::try_from(column.0).unwrap_or(u16::MAX));
            // The column is asked once for the need, and its cells are read where they lie.
            let held = view_of(frame.fill.store, source, place);
            rows.any(|row| cell_at(held, Count(row)) == wanted)
        };
        if holds {
            NeedVerdict::Holds
        } else {
            NeedVerdict::Empty
        }
    }

    /// Whether the table has a row in the file: made when its own need holds.
    fn table_decided(&mut self, table: MadeTableId) -> NeedVerdict {
        let frame = &mut *self.0;
        let ports = frame.ports;
        if ports.table_need(frame, table) == NeedVerdict::Empty {
            return NeedVerdict::Empty;
        }
        // SPIKE: a measuring worker books the fill this need starts (the table is not made yet for the file).
        let begun = (frame.state.measure.is_some() && table_of(frame.fill.store, table).is_none())
            .then(need_fill_begun);
        let made_now = made(
            frame.fill,
            &mut Asker {
                state: &mut *frame.state,
                found: &mut *frame.found,
                ports: frame.ports,
            },
            table,
        );
        let rows = table_of(frame.fill.store, table).map_or(0, |slot| slot.rows.count.0);
        if let Some(begun) = begun {
            need_fill_ended(frame.state, table, begun, Count(rows));
        }
        if made_now.is_ok() && rows > 0 {
            NeedVerdict::Holds
        } else {
            NeedVerdict::Empty
        }
    }
}
