//! What the `scan` use case promises: a rule set with no program scans a file, finds nothing and asks the language for nothing.

mod growth;
mod runs;

use super::interface::ScanUseCase as _;
use super::use_case::ScanUseCaseImpl;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::findings::types::{
    FoundCells, FoundRows, FoundTarget, InstructionCount, ScanCounters, WorkNotice,
};
use crate::domain::programs::service::scratch_for;
use crate::domain::programs::types::Programs;
use ryo_language::domain::facts::types::{
    FactAnswer, FactSource, FileFacts, FileWords, FilledColumns, Wanted,
};
use ryo_language::domain::node_table::service::{builder, finished};
use ryo_language::domain::node_table::types::NodeTable;
use ryo_language::domain::semantics::types::{RenderOutcome, RenderRequest};
use ryo_language::domain::symbols::types::ByteSlice;
use ryo_matching::inbound::use_cases::matching::use_case::MatchingUseCaseImpl;
use ryo_relations::domain::answers::types::AbsenceCause;
use ryo_relations::domain::relations::types::{Count, FileId, Relation};

/// A language's facts that count what the engine asks.
#[derive(Default)]
struct Counting(usize);

impl FactSource for Counting {
    fn fill(
        &mut self,
        _relation: Relation,
        _wanted: Wanted,
        _words: &mut FileWords,
        _out: &mut FilledColumns,
    ) -> FactAnswer {
        self.0 = self.0.saturating_add(1);
        FactAnswer::Refused(AbsenceCause::NotYetRead)
    }

    fn render(&mut self, _request: RenderRequest) -> RenderOutcome {
        RenderOutcome::NotApplicable
    }
}

/// Counts the rows a scan hands over.
#[derive(Default)]
struct Found(usize);

impl FoundRows for Found {
    fn found(&mut self, _target: FoundTarget, _row: FoundCells<'_>, _words: &FileWords) {
        self.0 = self.0.saturating_add(1);
    }

    fn noticed(&mut self, _notice: WorkNotice) {}
}

type Input = ();
/// The counters, the rows found and the fills asked.
type Value = (ScanCounters, usize, usize);
type Failure = crate::inbound::use_cases::scan::errors::ScanError;

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_rule_set_with_no_program_finds_nothing_and_asks_nothing"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value((
        ScanCounters {
            steps: InstructionCount(0),
            candidates: InstructionCount(0),
            made_rows: InstructionCount(0),
            fill_rows: InstructionCount(0),
            visited: InstructionCount(0),
        },
        0,
        0,
    )),
}];

fn subject((): Input) -> Result<Value, Failure> {
    let programs = Programs::default();
    let mut table = NodeTable::default();
    let mut writer = builder(&mut table, Count(1));
    let (mut facts, mut found) = (Counting::default(), Found::default());
    let file = FileFacts {
        table: finished(&mut writer),
        source: ByteSlice(b""),
        file: FileId(0),
        path: ryo_language::domain::files::types::SourcePath(""),
        facts: &mut facts,
    };
    let counters = ScanUseCaseImpl::new(std::sync::Arc::new(MatchingUseCaseImpl)).scan(
        &programs,
        file,
        &mut scratch_for(&programs),
        &mut found,
    )?;
    Ok((counters, found.0, facts.0))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
