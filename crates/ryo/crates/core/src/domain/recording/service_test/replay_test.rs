//! What a visit comes to once it ends: the record of a file the store kept is used again, and the record a visit keeps is made.

use super::*;
use crate::domain::handing::service::replay;
use crate::domain::recording::service::finish_file;
use crate::domain::workers::types::FileOrder;
use ryo_language::domain::elements::service::notes_mut;
use ryo_values::domain::files::types::{CorpusEntry, FileNumber, FileSize, Nanoseconds};
use ryo_values::domain::records::types::{FileOutcome, Ran, Record, Unreadable};
use ryo_values::domain::stored_facts::types::LanguageNumber;

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Uses again the record of a file the rules were evaluated on.
    ReplayRead,
    /// Ends the visit of a file whose record the run keeps and which a language wrote notes of.
    FinishKept,
}

/// What a row of the cases does.
type Input = Scenario;
/// A count, or several written one after the other as digits.
type Value = u64;
type Failure = ();

fn entry() -> CorpusEntry {
    CorpusEntry {
        executable: Default::default(),
        path: FilePath("a.rs".into()),
        size: FileSize(10),
        modified: Nanoseconds(5),
    }
}

fn unread() -> FileOutcome {
    FileOutcome::Unread {
        reason: Unreadable::NoLanguage,
        ran: None,
    }
}

fn ran() -> Ran {
    made_ran(
        &tables(),
        FileId(4),
        Digest([0; 16]),
        Vec::new(),
        ArenaBase(0),
    )
}

fn replay_scenario(outcome: FileOutcome) -> Value {
    let record = Record {
        executable: Default::default(),
        path: FilePath("a.rs".into()),
        size: FileSize(1),
        modified: Nanoseconds(1),
        digest: Digest([0; 16]),
        number: FileNumber(0),
        outcome,
    };
    let mut worker = tables();
    replay(
        &mut worker,
        FileId(3),
        FileOrder(0),
        &record,
        &entry(),
        Digest([1; 16]),
    );
    let fresh = &worker.fresh[0].record;
    let renewed = fresh.number == FileNumber(3) && fresh.digest == Digest([1; 16]);
    count(worker.unread.len()) * 100 + count(worker.fresh.len()) * 10 + u64::from(renewed)
}

fn finish_scenario() -> Result<Value, Failure> {
    let placed = placed()?;
    let setting = VisitSetting {
        noted: Noting::Wanted,
        ..setting(&placed)
    };
    let mut worker = tables();
    notes_mut(&mut worker.elements).0.push(9);
    worker.language = Some(LanguageNumber(0));
    finish_file(
        &mut worker,
        &setting,
        FileOrder(0),
        &entry(),
        Digest([1; 16]),
        FileNumber(3),
        Some(unread()),
    );
    Ok(count(worker.fresh.len()) * 100
        + count(worker.notes.len()) * 10
        + count(worker.partials.len()))
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    match scenario {
        Scenario::ReplayRead => Ok(replay_scenario(FileOutcome::Read(ran()))),
        Scenario::FinishKept => finish_scenario(),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_file_the_store_kept_is_renumbered_and_its_record_kept_again"),
        word: Word::Succeeds,
        input: || Scenario::ReplayRead,
        expect: Expect::Value(11),
    },
    Case {
        name: CaseName(
            "a_visit_ends_with_the_record_and_the_notes_when_the_run_keeps_its_findings",
        ),
        word: Word::Succeeds,
        input: || Scenario::FinishKept,
        expect: Expect::Value(110),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
