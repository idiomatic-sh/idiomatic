//! The `coverage` service's cases, on values alone.

use ryo_relations::domain::relations::types::Integer;
use ryo_relations::domain::relations::types::{CellKind, FileId, Relation, Value as Cell};

use super::service::{affected_ordinals, drop_uncertain, partial_of};
use super::types::{DeclaredFiles, Recovery};
use crate::domain::operating::errors::OperatingError;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
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
    /// The relations a file whose tokens are complete is read in part in, with and without a refusal of the reader.
    LexicalRecovery,
    /// The relations a file that is only a recovery is read in part in.
    EveryRelationRecovered,
    /// What a recovered file leaves of the rows of an absence check on byte literals.
    RecoveredByteLiterals,
    /// What a file read whole leaves of the same absence check.
    WholeSource,
    /// The ordinals of the files named by a partial source, over three declared files, as a bit set.
    Affected(&'static [Option<FileId>]),
    /// The same, over the number of declared files.
    AffectedOver(&'static [Option<FileId>], usize),
}

/// What a row of the cases does.
type Input = Scenario;
/// A yes or no as a number, or how many rows remain.
type Value = usize;
/// What the placing of a partial source is refused with.
type Failure = OperatingError;

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

fn subject(scenario: Scenario) -> Result<Value, Failure> {
    match scenario {
        Scenario::KeyedByFile => {
            let mut table = rows();
            drop_uncertain(
                &antijoin(0),
                &[CellKind::File, CellKind::Number],
                &mut table,
                &[Some(FileId(2))],
            );
            Ok(table.count.0)
        }
        Scenario::KeyedByName => {
            let mut table = rows();
            drop_uncertain(
                &antijoin(1),
                &[CellKind::File, CellKind::Number],
                &mut table,
                &[Some(FileId(2))],
            );
            Ok(table.count.0)
        }
        Scenario::Recovered => Ok(partial_of(
            &[],
            Recovery::SyntaxErrors,
            &[Relation::Defines, Relation::FilesystemFile],
        )
        .len()),
        Scenario::LexicalRecovery => Ok(lexical_recovery_keeps_explicit_reader_refusals_and_code_uncertainty()),
        Scenario::EveryRelationRecovered => Ok(every_wanted_relation_is_partial_when_only_a_recovery()),
        Scenario::RecoveredByteLiterals => Ok(recovered_byte_literals_refuse_absence()),
        Scenario::WholeSource => Ok(whole_source_keeps_the_same_result()),
        Scenario::Affected(files) => affected_ordinals(files, DeclaredFiles(3)).map(bit_set),
        Scenario::AffectedOver(files, declared) => {
            affected_ordinals(files, DeclaredFiles(declared)).map(bit_set)
        }
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
    Case {
        name: CaseName("lexical_recovery_keeps_explicit_reader_refusals_and_code_uncertainty"),
        word: Word::Succeeds,
        input: || Scenario::LexicalRecovery,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a_file_that_is_only_a_recovery_is_partial_in_every_wanted_relation"),
        word: Word::Succeeds,
        input: || Scenario::EveryRelationRecovered,
        expect: Expect::Value(6),
    },
    Case {
        name: CaseName("recovered_byte_literals_refuse_absence"),
        word: Word::Succeeds,
        input: || Scenario::RecoveredByteLiterals,
        expect: Expect::Value(0),
    },
    Case {
        name: CaseName("whole_source_keeps_the_same_result_of_an_absence_check"),
        word: Word::Succeeds,
        input: || Scenario::WholeSource,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("partial_project_coverage_names_each_affected_file_once"),
        word: Word::Succeeds,
        input: || Scenario::Affected(&[Some(FileId(2)), Some(FileId(0)), Some(FileId(2))]),
        expect: Expect::Value(0b101),
    },
    Case {
        name: CaseName("an_unlocated_partial_source_names_every_declared_file"),
        word: Word::Succeeds,
        input: || Scenario::Affected(&[None, Some(FileId(2))]),
        expect: Expect::Value(0b111),
    },
    Case {
        name: CaseName("a_partial_source_naming_no_file_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Affected(&[]),
        expect: Expect::Refusal(is_unplaceable),
    },
    Case {
        name: CaseName("a_partial_source_over_no_declared_file_is_refused"),
        word: Word::Refuses,
        input: || Scenario::AffectedOver(&[None], 0),
        expect: Expect::Refusal(is_unplaceable),
    },
    Case {
        name: CaseName("a_file_beyond_the_declared_files_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Affected(&[Some(FileId(3))]),
        expect: Expect::Refusal(is_unplaceable),
    },
    Case {
        name: CaseName("an_unlocated_source_beside_a_file_beyond_the_declared_files_is_refused"),
        word: Word::Refuses,
        input: || Scenario::Affected(&[None, Some(FileId(999))]),
        expect: Expect::Refusal(is_unplaceable),
    },
];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// The ordinals as a bit set: the ordinal `n` is the bit `1 << n`.
fn bit_set(ordinals: Vec<super::types::FileOrdinal>) -> Value {
    ordinals.iter().map(|ordinal| 1_usize << ordinal.0).sum()
}

/// Whether an error is the refusal to place a partial source on the declared files.
fn is_unplaceable(error: &OperatingError) -> ryo_values::cases::Selection {
    ryo_values::selects!(matches!(
        error,
        OperatingError::ObservationIncomplete { .. }
    ))
}

fn lexical_recovery_keeps_explicit_reader_refusals_and_code_uncertainty() -> Value {
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
    explicit.len()
}

fn every_wanted_relation_is_partial_when_only_a_recovery() -> Value {
    let wanted = [
        Relation::Comment,
        Relation::IndentRegion,
        Relation::Function,
    ];
    let mut found = 0;
    for recovery in [Recovery::SyntaxErrors, Recovery::Unparsed] {
        let incomplete = partial_of(&[], recovery, &wanted);
        for relation in wanted {
            assert!(incomplete.iter().any(|part| part.relation == relation));
            found += 1;
        }
    }
    found
}

fn recovered_byte_literals_refuse_absence() -> Value {
    use super::types::Lacking;
    let wanted = [Relation::BytesLiteral];
    let mut left = 0;
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
        left += absent.count.0;
    }
    left
}

fn whole_source_keeps_the_same_result() -> Value {
    let partial = partial_of(&[], Recovery::Whole, &[Relation::BytesLiteral]);
    assert!(partial.is_empty());
    let mut absent = rows();
    drop_uncertain(
        &antijoin(1),
        &[CellKind::File, CellKind::Number],
        &mut absent,
        &[],
    );
    absent.count.0
}

use ryo_values::domain::rows::types::Table;
