//! The `reporting` service's cases, on values alone.

use ryo_relations::domain::relations::types::{FileId, Integer, Value as Cell};

use ryo_values::domain::records::types::RowTotal;

use super::service::{order_found, report_refused, room_left, totals};
use super::types::RowCounts;
use crate::cases::{Case, CaseName, Expect, Word, run};
use {crate::domain::finish::types::ColumnNumber, crate::domain::finish::types::FileLine, crate::domain::finish::types::FoundRow, crate::domain::finish::types::Point, ryo_values::domain::runs::types::RuleNumber};
use ryo_language::domain::files::types::FilePath;
use ryo_language::domain::reasons::types::ToolMessage;
use ryo_values::domain::descriptions::types::LanguageName;
use ryo_values::domain::records::types::Unread;
use ryo_values::domain::records::types::Unreadable;
use ryo_values::domain::runs::types::ManifestRefused;
use ryo_values::domain::stored_facts::types::ReadFailure;

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Puts the findings of three files in the order of their paths.
    Ordered,
    /// Asks for the room left when more rows were shown than the limit.
    NoRoomLeft,
    /// Adds the findings of the cross-file steps to what the files counted.
    Counts,
    /// Puts the refused manifests `p2` and `p1` among the unread file `p3`: the order of the paths.
    RefusedAreSorted,
    /// The same: whether a refused manifest is unread for the reason the tool gave, and the file unread for no language.
    RefusedAreFailures,
}

/// What a row of the cases does.
type Input = Scenario;
/// The numbers the scenario answers, padded with zeros.
type Value = [u64; 3];
type Failure = ();

/// A finding of `file`, at line 3 and character 4, whose row is the number 7.
fn finding(file: u32) -> FoundRow {
    FoundRow {
        rule: RuleNumber(0),
        file: FileId(file),
        place: Point {
            line: FileLine(3),
            column: ColumnNumber(4),
        },
        row: vec![Cell::Number(Integer(7))],
    }
}

/// The unread files after the refused manifests `p2` and `p1` are put among the unread file `p3`.
fn reported() -> Vec<Unread> {
    let refused = |path: &str| ManifestRefused {
        language: LanguageName("rust".into()),
        path: FilePath(path.into()),
        reason: ToolMessage("bad".into()),
    };
    let mut unread = vec![Unread {
        path: FilePath("p3".into()),
        reason: Unreadable::NoLanguage,
    }];
    report_refused(&mut unread, &[refused("p2"), refused("p1")]);
    unread
}

fn subject(scenario: Input) -> Value {
    let paths = ["b.rs", "a.rs", "c.rs"].map(|path| FilePath(path.into()));
    let mut found = vec![finding(0), finding(1), finding(2)];
    match scenario {
        Scenario::Ordered => {
            order_found(&mut found, &paths);
            [0, 1, 2].map(|at| u64::from(found[at].file.0))
        }
        Scenario::NoRoomLeft => [room_left(RowTotal(5), RowTotal(7)).0, 0, 0],
        Scenario::Counts => {
            let files = RowCounts {
                total: RowTotal(5),
                shown: RowTotal(3),
            };
            let counted = totals(files, RowTotal(3), RowTotal(2));
            [counted.total.0, counted.shown.0, 0]
        }
        Scenario::RefusedAreSorted => {
            let unread = reported();
            [0, 1, 2].map(|at| {
                unread
                    .get(at)
                    .map_or(0, |each| u64::from(each.path.0.as_bytes()[1] - b'0'))
            })
        }
        Scenario::RefusedAreFailures => {
            let unread = reported();
            [
                u64::from(
                    unread.first().map(|each| &each.reason)
                        == Some(&Unreadable::Failed(ReadFailure("bad".into()))),
                ),
                u64::from(unread.get(2).map(|each| &each.reason) == Some(&Unreadable::NoLanguage)),
                0,
            ]
        }
    }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("findings_are_put_in_the_order_of_their_paths"),
        word: Word::Succeeds,
        input: || Scenario::Ordered,
        expect: Expect::Value([1, 0, 2]),
    },
    Case {
        name: CaseName("there_is_no_room_left_when_the_files_showed_more_than_the_limit"),
        word: Word::Succeeds,
        input: || Scenario::NoRoomLeft,
        expect: Expect::Value([0, 0, 0]),
    },
    Case {
        name: CaseName("every_finding_is_counted_and_those_up_to_the_room_are_shown"),
        word: Word::Succeeds,
        input: || Scenario::Counts,
        expect: Expect::Value([8, 5, 0]),
    },
    Case {
        name: CaseName(
            "refused_manifests_are_put_among_the_unread_files_in_the_order_of_the_paths",
        ),
        word: Word::Succeeds,
        input: || Scenario::RefusedAreSorted,
        expect: Expect::Value([1, 2, 3]),
    },
    Case {
        name: CaseName("a_refused_manifest_is_unread_for_the_reason_the_tool_gave"),
        word: Word::Succeeds,
        input: || Scenario::RefusedAreFailures,
        expect: Expect::Value([1, 1, 0]),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}
