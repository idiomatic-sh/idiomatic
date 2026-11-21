//! The types of the `dispatch` concept: the loop's frame as what answers a need's leaves.

use crate::domain::code::types::{NeedSite, RelationNumber, WordSlot};
use crate::domain::needs::types::{NeedFacts, NeedVerdict};
use crate::domain::programs::collections::instructions::interface::StepFrame;
use crate::domain::steps::types::MadeTableId;
use ryo_language::domain::node_table::service::kinds_present;
use ryo_relations::domain::relations::types::KindBits;

/// The loop's frame, as what a need's formula asks: its decisions are kept in the frame's state, a kind's presence is the
/// node table's, a table's row is the table's.
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
        let present = kinds_present(self.0.file.table)
            .0
            .get(usize::from(kind.0 / 64))
            .is_some_and(|word| (word.0 >> (kind.0 % 64)) & 1 == 1);
        if present {
            NeedVerdict::Holds
        } else {
            NeedVerdict::Empty
        }
    }

    fn relation_row(
        &mut self,
        _relation: RelationNumber,
        _column: WordSlot,
        _constant: WordSlot,
    ) -> NeedVerdict {
        // A relation's rows are the fill side's: until its seam answers, nothing is known and the unit is evaluated.
        NeedVerdict::Holds
    }

    fn table_row(&mut self, table: MadeTableId) -> NeedVerdict {
        let rows = self
            .0
            .state
            .tables
            .get(usize::try_from(table.0).unwrap_or(usize::MAX))
            .map_or(0, |slot| slot.rows.count.0);
        if rows > 0 {
            NeedVerdict::Holds
        } else {
            NeedVerdict::Empty
        }
    }
}
