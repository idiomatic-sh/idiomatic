//! The `coverage` service's cases, on values alone.

use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::{CellKind, FileId, Relation, Value as Cell};

use super::service::{drop_uncertain, partial_of};
use super::types::Recovery;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::rows::service::push_row;

use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Operation;

/// What a row of the cases does.
#[derive(Clone, Copy)]
enum Scenario {
    /// Drops from the answer of an antijoin keyed by file the rows that missing rows of file 2 could change.
    KeyedByFile,
    /// Drops from the answer of an antijoin keyed by a name the rows that missing rows of any file could change.
    KeyedByName,
    /// Says why a file with syntax errors is read in part.
    Recovered,
}

/// What a row of the cases does.
type Input = Scenario;
/// A yes or no as a number, or how many rows remain.
type Value = usize;
type Failure = ();

fn antijoin(key: u32) -> Operation {
    Operation::Antijoin {
        left: PlanInput::Step(StepId(0)),
        right: PlanInput::Step(StepId(1)),
        left_on: vec![ColumnIndex(key)],
        right_on: vec![ColumnIndex(key)],
    }
}

/// Two rows, one of file 1 and one of file 2, under a column of files and a column of numbers.
fn rows() -> Table {
    let mut table = Table {
        kinds: vec![CellKind::File, CellKind::Number],
        ..Table::default()
    };
    push_row(
        &mut table,
        &[Cell::File(FileId(1)), Cell::Number(Integer(7))],
    );
    push_row(
        &mut table,
        &[Cell::File(FileId(2)), Cell::Number(Integer(7))],
    );
    table
}

fn subject(scenario: Scenario) -> Value {
    match scenario {
        Scenario::KeyedByFile => {
            let mut table = rows();
            drop_uncertain(
                &antijoin(0),
                &[CellKind::File, CellKind::Number],
                &mut table,
                &[Some(FileId(2))],
            );
            table.count.0
        }
        Scenario::KeyedByName => {
            let mut table = rows();
            drop_uncertain(
                &antijoin(1),
                &[CellKind::File, CellKind::Number],
                &mut table,
                &[Some(FileId(2))],
            );
            table.count.0
        }
        Scenario::Recovered => partial_of(
            &[],
            Recovery::SyntaxErrors,
            &[Relation::Defines, Relation::FilesystemFile],
        )
        .len(),
    }
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_row_keyed_by_a_file_is_dropped_only_if_its_file_could_be_missing_rows"),
        word: Word::Succeeds,
        input: || Scenario::KeyedByFile,
        expect: Expect::Value(1),
    },
    Case {
        name: CaseName("a_row_with_no_file_key_is_dropped_whichever_file_could_be_missing_rows"),
        word: Word::Succeeds,
        input: || Scenario::KeyedByName,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("a_file_with_syntax_errors_is_read_in_part_in_the_relations_of_code_only"),
        word: Word::Succeeds,
        input: || Scenario::Recovered,
        expect: Expect::Value(1),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |input| Ok(subject(input)));
}

#[test]
fn lexical_recovery_keeps_explicit_reader_refusals_and_code_uncertainty() {
    use ryo_relations::domain::relations::types::Missing;
    use ryo_values::domain::stored_facts::types::{PartialCause, PartialScope, ReadInPart};

    let wanted = [
        Relation::Comment,
        Relation::IndentRegion,
        Relation::Function,
    ];
    let lexical = partial_of(&[], Recovery::LexicallyCompleteSyntaxErrors, &wanted);
    assert_eq!(lexical.len(), 1);
    assert_eq!(lexical[0].relation, Relation::Function);

    let refused = ReadInPart {
        relation: Relation::Comment,
        cause: PartialCause("reader stopped before EOF".into()),
        lacking: Missing::Any,
        at: PartialScope::Repository,
    };
    let explicit = partial_of(&[refused], Recovery::LexicallyCompleteSyntaxErrors, &wanted);
    assert_eq!(explicit.len(), 2);
    assert!(
        explicit
            .iter()
            .any(|part| part.relation == Relation::Comment)
    );
    assert!(
        explicit
            .iter()
            .any(|part| part.relation == Relation::Function)
    );

    for recovery in [Recovery::SyntaxErrors, Recovery::Unparsed] {
        let incomplete = partial_of(&[], recovery, &wanted);
        for relation in wanted {
            assert!(incomplete.iter().any(|part| part.relation == relation));
        }
    }
}

#[test]
fn recovered_byte_literals_refuse_absence_while_whole_source_keeps_the_same_result() {
    use super::types::Lacking;
    let wanted = [Relation::BytesLiteral];
    for recovery in [
        Recovery::SyntaxErrors,
        Recovery::LexicallyCompleteSyntaxErrors,
        Recovery::Unparsed,
    ] {
        let partial = partial_of(&[], recovery, &wanted);
        assert_eq!(partial.len(), 1);
        assert_eq!(partial[0].relation, Relation::BytesLiteral);
        assert_eq!(partial[0].lacking, Lacking::Any);
        let mut absent = rows();
        drop_uncertain(
            &antijoin(1),
            &[CellKind::File, CellKind::Number],
            &mut absent,
            &partial.iter().map(|part| part.at).collect::<Vec<_>>(),
        );
        assert_eq!(absent.count.0, 0, "{recovery:?}");
    }
    let partial = partial_of(&[], Recovery::Whole, &wanted);
    assert!(partial.is_empty());
    let mut absent = rows();
    drop_uncertain(
        &antijoin(1),
        &[CellKind::File, CellKind::Number],
        &mut absent,
        &[],
    );
    assert_eq!(absent.count.0, 2);
}

use ryo_values::domain::rows::types::Table;

#[test]
fn partial_project_coverage_names_only_affected_files_and_refuses_unplaceable_roots() {
    use super::service::affected_ordinals;
    assert_eq!(
        affected_ordinals(&[Some(FileId(2)), Some(FileId(0)), Some(FileId(2))], 3).unwrap(),
        [0, 2]
    );
    assert_eq!(
        affected_ordinals(&[None, Some(FileId(2))], 3).unwrap(),
        [0, 1, 2]
    );
    assert!(affected_ordinals(&[], 3).is_err());
    assert!(affected_ordinals(&[None], 0).is_err());
    assert!(affected_ordinals(&[Some(FileId(3))], 3).is_err());
    assert!(affected_ordinals(&[None, Some(FileId(999))], 3).is_err());
}
