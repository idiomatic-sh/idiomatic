//! Test support of the `programs` concept: a word, or a program, run against a small per-file state and the fill side's
//! store, with the rows it found collected.

use crate::domain::code::types::CodeEntry;
use crate::domain::findings::types::{
    FoundCells, FoundRows, FoundTarget, InstructionCount, WorkNotice,
};
use crate::domain::made_tables::service::{clear_rows, write};
use crate::domain::made_tables::types::{FillFrame, FillStore, MadeState, PatternAsk, TableSlot};
use crate::domain::memos::types::MemoId;
use crate::domain::needs::types::NeedVerdict;
use crate::domain::programs::frames::types::{LoopEntry, RunMode, StepFrame};
use crate::domain::programs::types::ProgramsEngine;
use crate::domain::scratch::types::LoopState;
use crate::domain::stages::types::BatchValue;
use crate::domain::steps::types::MadeTableId;
use ryo_language::domain::facts::service::file_words;
use ryo_language::domain::facts::types::{
    CellWord, FactAnswer, FactSource, FileWords, FilledColumns, Wanted,
};
use ryo_language::domain::node_table::types::NodeTable;
use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
use ryo_operators::domain::patterns::types::PatternMatcher;
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::{Count, FileId, Relation};

/// The file the fixtures scan.
pub(crate) const FILE: FileId = FileId(7);

/// A language that fills nothing.
pub(crate) struct NoFacts;

impl FactSource for NoFacts {
    fn fill(
        &mut self,
        _relation: Relation,
        _wanted: Wanted,
        _words: &mut FileWords,
        _out: &mut FilledColumns,
    ) -> FactAnswer {
        FactAnswer::Refused(AbsenceCause::NoSuchConstruct)
    }

    fn render(&mut self, _request: RenderRequest) -> RenderOutcome {
        RenderOutcome::NotApplicable
    }
}

/// The rows found, each as its cells, and the notices given.
#[derive(Debug, Default)]
pub(crate) struct Collected {
    /// The rows, in the order found.
    pub(crate) rows: Vec<Vec<u64>>,
    /// The notices, in the order given.
    pub(crate) notices: Vec<WorkNotice>,
}

impl FoundRows for Collected {
    fn found(&mut self, _target: FoundTarget, row: FoundCells<'_>, _words: &FileWords) {
        self.rows.push(row.0.iter().map(|cell| cell.0).collect());
    }

    fn noticed(&mut self, notice: WorkNotice) {
        self.notices.push(notice);
    }
}

/// What a case holds between its setup and its run: the loop's state, and the fill side's store.
#[derive(Debug, Default)]
pub(crate) struct Held {
    /// The loop's own per-file state.
    pub(crate) state: LoopState,
    /// The fill side's store.
    pub(crate) store: FillStore,
}

/// A state of `registers` registers, and an empty store.
pub(crate) fn held(registers: usize) -> Held {
    Held {
        state: LoopState {
            registers: vec![CellWord(0); registers],
            ..LoopState::default()
        },
        store: FillStore::default(),
    }
}

/// Makes table `table` of the store hold `rows`, each of `width` cells, as a table made for the store's file: what a fill
/// would have left. Test code alone writes a store's field; the loop reads it through the seam.
pub(crate) fn table(case: &mut Held, table: usize, width: usize, rows: &[&[u64]]) {
    if case.store.tables.len() <= table {
        case.store.tables.resize_with(table + 1, TableSlot::default);
    }
    let stamp = case.store.stamp;
    let slot = &mut case.store.tables[table];
    slot.stamp = stamp;
    slot.state = MadeState::Made;
    clear_rows(&mut slot.rows, Count(width));
    for row in rows {
        let sources: Vec<BatchValue<'_>> = row
            .iter()
            .map(|cell| BatchValue::Constant(CellWord(*cell)))
            .collect();
        write(&mut slot.rows, &sources, Count(1));
    }
}

/// A loop that runs nothing: the ports of a case that runs one item alone, which calls no port.
#[derive(Clone, Copy, Debug)]
pub(crate) struct Idle;

impl LoopEntry for Idle {
    fn run(&self, _: &mut StepFrame<'_, '_>, _: CodeEntry, _: RunMode) -> InstructionCount {
        InstructionCount(0)
    }

    fn pattern(&self, _: &mut StepFrame<'_, '_>, _: PatternAsk<'_>) -> MemoId {
        MemoId(0)
    }

    fn table_need(&self, _: &mut StepFrame<'_, '_>, _: MadeTableId) -> NeedVerdict {
        NeedVerdict::Holds
    }
}

/// Runs `each` over a frame of `programs` and the case's state and store on the fixture file, and answers what it answered
/// and what was found.
pub(crate) fn framed<T>(
    programs: &ProgramsEngine,
    case: &mut Held,
    ports: &dyn LoopEntry,
    each: impl FnOnce(&mut StepFrame<'_, '_>) -> T,
) -> (T, Collected) {
    let (nodes, mut facts, mut found) = (NodeTable::default(), NoFacts, Collected::default());
    let mut words = file_words(&programs.words);
    let mut fill = FillFrame {
        programs,
        nodes: &nodes,
        file: FILE,
        text: ByteSlice(b""),
        facts: &mut facts,
        words: &mut words,
        store: &mut case.store,
        matcher: PatternMatcher(&MatchingUseCaseImpl),
    };
    let answer = each(&mut StepFrame {
        fill: &mut fill,
        state: &mut case.state,
        found: &mut found,
        ports,
    });
    (answer, found)
}
