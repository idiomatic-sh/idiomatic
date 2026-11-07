//! What the schemas of this output print: a violation of a run and a record of the lock, each as the one row every
//! command prints, as JSON reads it, and where a row is as a compiler prints it.

use asymptote_core::domain::operations::types::{Count, Day, Provenance, Reason, Record};
use asymptote_core::domain::violations::types::{
    ColumnNumber, Fingerprint, Fingerprinted, Id, Key, Level, LineNumber, MessageText, Place,
    RepoPath, RuleId, Standing, Suppression, ToolId, Violation,
};

use crate::cases::{Case, CaseName, Expect, Word, run};

/// Which row is printed.
#[derive(Clone, Copy, Debug)]
enum Row {
    /// A violation at a line and column, its fingerprint known.
    Violation,
    /// A violation of a whole file, its fingerprint unknown.
    WholeFile,
    /// A record of the lock.
    Record,
}

/// The row asked for.
type Input = Row;
/// Where it is, then its JSON.
type Value = std::borrow::Cow<'static, str>;
/// Printing a row cannot fail.
type Failure = std::convert::Infallible;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_violation_prints_its_place_and_its_fingerprint"),
        word: Word::Succeeds,
        input: || Row::Violation,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            r#"src/a.rs:3:2 {"path":"src/a.rs","line":3,"column":2,"tool":"lint","rule":"r","message":"m","status":"new","level":"warning","fingerprint":"00000000000000ff"}"#,
        )),
    },
    Case {
        name: CaseName("a_violation_of_a_whole_file_has_no_place_and_no_fingerprint"),
        word: Word::Succeeds,
        input: || Row::WholeFile,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            r#"src/a.rs {"path":"src/a.rs","line":null,"column":null,"tool":"lint","rule":"r","message":"m","status":"new","level":"warning"}"#,
        )),
    },
    Case {
        name: CaseName("a_record_prints_its_fingerprint_and_its_count"),
        word: Word::Succeeds,
        input: || Row::Record,
        expect: Expect::Value(std::borrow::Cow::Borrowed(
            r#"src/b.rs {"path":"src/b.rs","line":null,"column":null,"tool":"lint","rule":"r","message":"m","status":"accepted","level":"error","fingerprint":"00000000000000ff","count":2}"#,
        )),
    },
];

/// The violation every row of a run starts from, at `place`.
fn found(place: Place, fingerprint: Fingerprinted) -> Violation {
    Violation {
        path: RepoPath("src/a.rs".into()),
        place,
        tool: ToolId("lint".into()),
        rule: RuleId("r".into()),
        level: Level::Warning,
        suppression: Suppression::Unsuppressed,
        message: MessageText("m".into()),
        fingerprint,
    }
}

/// Prints the row asked for: where it is, then its JSON.
#[expect(
    clippy::unnecessary_wraps,
    reason = "every part's runner takes a subject that answers a Result, as `run` states it; these rows only succeed"
)]
fn subject(input: Input) -> Result<Value, Failure> {
    let row = match input {
        Row::Violation => super::violation(
            &found(
                Place::At(LineNumber(3), ColumnNumber(2)),
                Fingerprinted::Known(Fingerprint(255)),
            ),
            "new",
        ),
        Row::WholeFile => super::violation(&found(Place::File, Fingerprinted::Unknown), "new"),
        Row::Record => super::record(
            &Record {
                id: Id("x".into()),
                key: Key {
                    tool: ToolId("lint".into()),
                    rule: RuleId("r".into()),
                    path: RepoPath("src/b.rs".into()),
                    level: Level::Error,
                    standing: Standing::Reported,
                    fingerprint: Fingerprint(255),
                },
                count: Count(2),
                message: MessageText("m".into()),
                since_rev: Provenance::Unknown,
                since: Day("2026-10-28".into()),
                reasons: vec![Reason("why".into())],
            },
            "accepted",
        ),
    };
    let json = serde_json::to_string(&row).expect("a row serializes");
    Ok(std::borrow::Cow::Owned(format!(
        "{} {json}",
        super::place(&row)
    )))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
