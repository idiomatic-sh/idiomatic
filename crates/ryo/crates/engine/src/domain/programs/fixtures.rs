//! Test support of the `programs` concept: a word run against a small per-file state, with the rows it found collected.

use crate::domain::findings::types::{FoundCells, FoundRows, FoundTarget, WorkNotice};
use crate::domain::made_tables::service::{clear_rows, write};
use crate::domain::programs::collections::instructions::interface::StepFrame;
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::scratch::types::{LoopState, TableSlot};
use crate::domain::stages::types::BatchValue;
use ryo_language::domain::facts::types::{
    CellWord, FactAnswer, FactSource, FileFacts, FileWords, FilledColumns,
};
use ryo_language::domain::node_table::types::NodeTable;
use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::{Count, FileId, Relation};

/// The file the fixtures scan.
pub(crate) const FILE: FileId = FileId(7);

/// A language that fills nothing.
struct NoFacts;

impl FactSource for NoFacts {
    fn fill(
        &mut self,
        _relation: Relation,
        _words: &mut FileWords,
        _out: &mut FilledColumns,
    ) -> FactAnswer {
        FactAnswer::Refused(AbsenceCause::NoSuchConstruct)
    }

    fn render(&mut self, _request: RenderRequest) -> RenderOutcome {
        RenderOutcome::NotApplicable
    }
}

/// The rows found, each as its cells, and how many notices came.
#[derive(Debug, Default)]
pub(crate) struct Collected {
    /// The rows, in the order found.
    pub(crate) rows: Vec<Vec<u64>>,
    /// How many notices came.
    pub(crate) notices: usize,
}

impl FoundRows for Collected {
    fn found(&mut self, _target: FoundTarget, row: FoundCells<'_>, _words: &FileWords) {
        self.rows.push(row.0.iter().map(|cell| cell.0).collect());
    }

    fn noticed(&mut self, _notice: WorkNotice) {
        self.notices += 1;
    }
}

/// A per-file state of `registers` registers.
pub(crate) fn state(registers: usize) -> LoopState {
    LoopState {
        registers: vec![CellWord(0); registers],
        ..LoopState::default()
    }
}

/// Makes table `table` of the state hold `rows`, each of `width` cells, for the state's file.
pub(crate) fn table(held: &mut LoopState, table: usize, width: usize, rows: &[&[u64]]) {
    if held.tables.len() <= table {
        held.tables.resize_with(table + 1, TableSlot::default);
    }
    let slot = &mut held.tables[table];
    slot.stamp = held.stamp;
    clear_rows(&mut slot.rows, Count(width));
    for row in rows {
        let sources: Vec<BatchValue<'_>> = row
            .iter()
            .map(|cell| BatchValue::Constant(CellWord(*cell)))
            .collect();
        write(&mut slot.rows, &sources, Count(1));
    }
}

/// Runs `each` over a frame of `programs` and `held` on the fixture file, and answers what it answered and what was found.
pub(crate) fn framed<T>(
    programs: &ProgramsEngine,
    held: &mut LoopState,
    each: impl FnOnce(&mut StepFrame<'_, '_>) -> T,
) -> (T, Collected) {
    let (table, mut facts, mut found) = (NodeTable::default(), NoFacts, Collected::default());
    let mut file = FileFacts {
        table: &table,
        source: ByteSlice(b""),
        file: FILE,
        facts: &mut facts,
    };
    let answer = each(&mut StepFrame {
        file: &mut file,
        state: held,
        programs,
        found: &mut found,
    });
    (answer, found)
}
