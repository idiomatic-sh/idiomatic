//! A recognised ordered existence answers as its declined twin through the real compiled owner, whole and read in part.
use super::super::route_controls::fixtures::{MeaningCalls, Supplied, run_read};
use super::fixtures::first_of_each_line;
use crate::application::file_dag::program::rule_outcome::types::RootOutcome;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::coverage::types::{Lacking, Partial};
use ryo_relations::domain::relations::{
    service::{kinds, node_id},
    types::{CellKind, FileId, Flag, Integer, Offset, Position, Relation, Span, Value as Cell},
};
use ryo_values::domain::stored_facts::types::PartialCause;
use ryo_values::domain::{
    plan_parts::types::ColumnIndex,
    rows::types::{RowCount, Table},
};
use std::borrow::Cow;
use std::sync::Arc;

/// What an answer was: complete, partial in these files, or another outcome.
#[derive(Debug, PartialEq)]
enum Answer {
    Complete,
    Partial(Cow<'static, [Option<FileId>]>),
    Other,
}

/// What the recognised plan and its declined twin answered, and how often each entered the
/// ordered meaning.
#[derive(Debug, PartialEq)]
struct Observed {
    recognised: Answer,
    /// Whether the twin answered exactly what the recognised plan answered.
    twin_agrees: bool,
    ordered_entered: bool,
    twin_entered: usize,
}

/// Whether the source location relation is read in part (partial in the file's publication).
type Input = bool;
type Value = Observed;
type Failure = String;

// A recognised ordered existence over a computed source answers what its declined twin answers,
// whole and read in part: read in part, it is partial in the file and in the repository.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_recognised_ordered_existence_read_in_part_answers_as_its_declined_twin"),
        word: Word::Succeeds,
        input: || true,
        expect: Expect::Value(Observed {
            recognised: Answer::Partial(Cow::Borrowed(&[None, Some(FileId(0))])),
            twin_agrees: true,
            ordered_entered: true,
            twin_entered: 0,
        }),
    },
    Case {
        name: CaseName("a_recognised_ordered_existence_read_whole_answers_as_its_declined_twin"),
        word: Word::Succeeds,
        input: || false,
        expect: Expect::Value(Observed {
            recognised: Answer::Complete,
            twin_agrees: true,
            ordered_entered: true,
            twin_entered: 0,
        }),
    },
];

/// Reads the first location of each line through the recognised plan and its declined twin.
fn subject(read_in_part: Input) -> Result<Value, Failure> {
    let schema = kinds(Relation::SourceLocation).to_vec();
    // Two locations of file 0 on the same line, at bytes 5 and 1.
    let cells = [(1, 5), (2, 1)]
        .into_iter()
        .flat_map(|(row, byte)| {
            schema
                .iter()
                .enumerate()
                .map(move |(column, kind)| match kind {
                    CellKind::Node => Cell::Node(node_id(FileId(0), Position(row))),
                    CellKind::File => Cell::File(FileId(0)),
                    CellKind::Span => Cell::Span(Span {
                        start: Offset(0),
                        end: Offset(0),
                    }),
                    CellKind::Number => Cell::Number(Integer(if column == 5 { byte } else { 1 })),
                    CellKind::Symbol => {
                        Cell::Symbol(ryo_relations::domain::terms::types::Symbol(0))
                    }
                    CellKind::Flag => Cell::Flag(Flag::Lowered),
                })
        })
        .collect();
    let source = Table {
        kinds: schema,
        cells,
        count: RowCount(2),
    };
    let partials = [Partial {
        relation: Relation::SourceLocation,
        cause: PartialCause("partial provider fixture".into()),
        lacking: Lacking::Any,
        at: ryo_execution_domain::domain::coverage::types::PartialPlace::File(FileId(0)),
    }];
    let publication: &[Partial] = if read_in_part { &partials } else { &[] };
    // Each answer as shown in full, and how many times the ordered meaning was entered for it.
    let answer = |kept: Vec<ColumnIndex>| {
        let calls = Arc::new(MeaningCalls::default());
        let outcome = run_read(
            first_of_each_line(kept),
            &[(Relation::SourceLocation, source.clone())],
            (&[], publication),
            Supplied::Computed,
            Arc::clone(&calls),
        );
        let (shown, answer) = match outcome {
            RootOutcome::Partial { table, affected } => (
                format!("partial {table:?} {affected:?}"),
                Answer::Partial(Cow::Owned(affected)),
            ),
            RootOutcome::Complete(table) => (format!("complete {table:?}"), Answer::Complete),
            RootOutcome::Failed(error) => (format!("failed {error:?}"), Answer::Other),
            RootOutcome::Unknown(error) => (format!("unknown {error:?}"), Answer::Other),
            RootOutcome::Skipped(_) | RootOutcome::NoQuery | RootOutcome::Deferred { .. } => {
                ("other".to_owned(), Answer::Other)
            }
        };
        (
            shown,
            answer,
            calls.ordered.load(std::sync::atomic::Ordering::Relaxed),
        )
    };
    let (recognised_shown, recognised, ordered) = answer(vec![]);
    let (twin_shown, _, declined) = answer(vec![ColumnIndex(0), ColumnIndex(1)]);
    Ok(Observed {
        recognised,
        twin_agrees: recognised_shown == twin_shown,
        // The plan is recognised and its ordered meaning entered; the twin is declined and
        // never enters it.
        ordered_entered: ordered > 0,
        twin_entered: declined,
    })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
