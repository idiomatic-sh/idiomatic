//! The types of the `executing` concept: the loop as what answers the fill side.

use crate::domain::executing::budget::service::fill_asked;
use crate::domain::executing::measuring::service::{left, table_entered};
use crate::domain::filled::errors::FilledError;
use crate::domain::findings::types::FoundRows;
use crate::domain::made_tables::types::{
    FillFrame, PatternAsk, PatternFound, PatternRows, TableNeed,
};
use crate::domain::memos::service::{row_count, rows};
use crate::domain::needs::types::NeedVerdict;
use crate::domain::programs::frames::types::{LoopEntry, StepFrame};
use crate::domain::scratch::types::LoopState;
use crate::domain::steps::types::MadeTableId;
use ryo_language::domain::facts::types::CellWord;

/// The loop's half of a frame, as what the fill side asks back during one call of the seam: the rows of a pattern for a
/// key, and whether a table it reaches may have a row. It is handed the fill side's frame back, so a pattern's program and
/// a need's formula run through the one loop and the one store.
pub(crate) struct Asker<'a> {
    /// The loop's own per-file state.
    pub(crate) state: &'a mut LoopState,
    /// Where found rows and notices go.
    pub(crate) found: &'a mut dyn FoundRows,
    /// The loop's entries.
    pub(crate) ports: &'a dyn LoopEntry,
}

impl PatternRows for Asker<'_> {
    fn table_need(
        &mut self,
        frame: &mut FillFrame<'_>,
        table: MadeTableId,
    ) -> Result<TableNeed, FilledError> {
        let mut step = StepFrame {
            fill: frame,
            state: &mut *self.state,
            found: &mut *self.found,
            ports: self.ports,
        };
        Ok(match self.ports.table_need(&mut step, table) {
            NeedVerdict::Holds => TableNeed::Rows,
            NeedVerdict::Empty => TableNeed::Empty,
        })
    }

    fn pattern_rows(
        &mut self,
        frame: &mut FillFrame<'_>,
        asked: PatternAsk<'_>,
        out: &mut Vec<CellWord>,
    ) -> Result<PatternFound, FilledError> {
        let mut step = StepFrame {
            fill: frame,
            state: &mut *self.state,
            found: &mut *self.found,
            ports: self.ports,
        };
        let answer = self.ports.pattern(&mut step, asked);
        fill_asked(self.state, asked.asker, answer);
        out.extend_from_slice(rows(&self.state.memos, answer));
        Ok(if row_count(&self.state.memos, answer).0 > 0 {
            PatternFound::Rows
        } else {
            PatternFound::Nothing
        })
    }

    fn table_started(&mut self, table: MadeTableId) {
        table_entered(self.state, table);
    }

    fn table_ended(&mut self, _: MadeTableId) {
        left(self.state);
    }
}
