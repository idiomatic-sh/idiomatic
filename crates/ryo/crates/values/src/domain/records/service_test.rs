//! The `records` service's cases, on values alone.

use super::service::{is_as_it_was, keep, lent, owned};
use super::types::{
    Cell, CellText, FileOutcome, Freshness, Record, RenderedText, RowsTextAt, RowsTextView,
    TextRange, Unreadable,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::files::types::{CorpusEntry, FileNumber, FileSize, Nanoseconds};
use crate::domain::stored_facts::types::Digest;
use ryo_language::domain::files::types::FilePath;

/// What a row of the cases asks of the service.
#[derive(Clone, Copy)]
enum Scenario {
    /// A file the system dates and sizes as before.
    Same,
    /// A file that is another size.
    OtherSize,
    /// A file written since.
    WrittenSince,
    /// A file the system does not date.
    Undated,
    /// Cells kept in the text of their rows, lent back and owned: each is the cell it was.
    KeptAndLent,
    /// A cell whose range the text does not hold, lent.
    OutOfText,
}

/// What a row of the cases asks of the service.
type Input = Scenario;
/// Whether the answer was the one a token gives.
type Value = usize;
type Failure = ();

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_file_dated_and_sized_as_before_is_as_it_was"),
        word: Word::Succeeds,
        input: || Scenario::Same,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_file_of_another_size_has_changed"),
        word: Word::Succeeds,
        input: || Scenario::OtherSize,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_file_written_since_has_changed"),
        word: Word::Succeeds,
        input: || Scenario::WrittenSince,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_file_the_system_does_not_date_has_always_changed"),
        word: Word::Succeeds,
        input: || Scenario::Undated,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_kept_cell_is_lent_and_owned_as_the_cell_it_was"),
        word: Word::Succeeds,
        input: || Scenario::KeptAndLent,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_range_the_text_does_not_hold_lends_nothing"),
        word: Word::Succeeds,
        input: || Scenario::OutOfText,
        expect: Expect::Value(1),
    },
];

/// Whether cells kept in one text, lent back and owned, are the cells they were.
fn kept_and_lent() -> bool {
    let cells = [
        Cell::File(CellText("src/a.rs")),
        Cell::Text(CellText("main")),
        Cell::Number(ryo_relations::domain::relations::types::Integer(-7)),
        Cell::Span(ryo_relations::domain::relations::types::Span {
            start: ryo_relations::domain::relations::types::Offset(3),
            end: ryo_relations::domain::relations::types::Offset(4),
        }),
    ];
    let mut text = RenderedText::default();
    let kept: Vec<Cell<TextRange>> = cells.iter().map(|cell| keep(&mut text, *cell)).collect();
    let back: Vec<Cell> = kept
        .iter()
        .map(|cell| owned(lent(RowsTextView(&text.0), *cell)))
        .collect();
    text.0 == "src/a.rsmain" && back == cells.map(owned)
}

fn record(modified: u64) -> Record {
    Record {
        executable: Default::default(),
        path: FilePath("a".into()),
        size: FileSize(5),
        modified: Nanoseconds(modified),
        digest: Digest([0; 16]),
        number: FileNumber(0),
        outcome: FileOutcome::Unread {
            reason: Unreadable::NoLanguage,
            ran: None,
        },
    }
}

fn entry(size: u64, modified: u64) -> CorpusEntry {
    CorpusEntry {
        executable: Default::default(),
        path: FilePath("a".into()),
        size: FileSize(size),
        modified: Nanoseconds(modified),
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let answered = match scenario {
        Scenario::Same => is_as_it_was(&record(9), &entry(5, 9)) == Freshness::AsItWas,
        Scenario::OtherSize => is_as_it_was(&record(9), &entry(6, 9)) == Freshness::Changed,
        Scenario::WrittenSince => is_as_it_was(&record(9), &entry(5, 10)) == Freshness::Changed,
        Scenario::Undated => is_as_it_was(&record(0), &entry(5, 0)) == Freshness::Changed,
        Scenario::KeptAndLent => kept_and_lent(),
        Scenario::OutOfText => {
            lent(
                RowsTextView("ab"),
                Cell::Text(TextRange {
                    start: RowsTextAt(1),
                    end: RowsTextAt(9),
                }),
            ) == Cell::Text(CellText(""))
        }
    };
    Ok(usize::from(answered))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn executable_observation_invalidates_same_size_and_timestamp_in_both_directions() {
    use crate::domain::files::types::ExecutablePermission::{Posix, Unknown, Unsupported};
    for (before, after) in [
        (Posix(false), Posix(true)),
        (Posix(true), Posix(false)),
        (Unknown, Posix(false)),
        (Posix(false), Unknown),
        (Unsupported, Unknown),
    ] {
        let mut kept = record(7);
        kept.executable = before;
        let mut file = entry(5, 7);
        file.executable = before;
        assert_eq!(
            super::service::is_as_it_was(&kept, &file),
            crate::domain::records::types::Freshness::AsItWas
        );
        file.executable = after;
        assert_eq!(
            super::service::is_as_it_was(&kept, &file),
            crate::domain::records::types::Freshness::Changed
        );
    }
}
