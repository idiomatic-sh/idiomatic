//! Every provider kind reaches the required join capability, and empty or failed lefts never build a right index.
use super::super::super::super::super::*;
use super::super::super::cases::fixtures::{node, table_of, tree_rows, wide_tree};
use super::super::super::*;
use super::super::fixtures::Witness;
use super::fixtures::checked_refusal;
use crate::cases::{Case, CaseName, Expect, Word, run};
#[cfg(test)]
use fixtures::execution::*;
#[cfg(test)]
use fixtures::provider::*;
use ryo_execution_domain::domain::table_cells::types::TreeForm;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Integer, Value as Cell};
use ryo_relations::domain::relations::types::{ColumnsView, FactColumn};
use ryo_values::domain::compact_table::service::admit_compact;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::stored_facts::types::KeptWords;
use ryo_values::domain::table_words::service::{encode_table, pack};
use ryo_values::domain::table_words::types::{DecodeMode, WordSlice};
use std::sync::Arc;
use std::sync::atomic::Ordering;
fn computed_stored_compact_and_tree_providers_reach_the_same_required_capability() {
    let left = table_of(&[CellKind::Number], &[&[Cell::Number(Integer(7))]]);
    let right = table_of(
        &[CellKind::Number, CellKind::Number],
        &[
            &[Cell::Number(Integer(7)), Cell::Number(Integer(1))],
            &[Cell::Number(Integer(7)), Cell::Number(Integer(2))],
        ],
    );
    let op = operation(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
    let columns = [FactColumn(vec![7, 7]), FactColumn(vec![1, 2])];
    for rows in [
        Rows::Computed(&right),
        Rows::Stored(ColumnsView {
            kinds: &[CellKind::Number; 2],
            columns: &columns,
        }),
    ] {
        checked_refusal(&op, [Rows::Computed(&left), rows], &mut Symbols::default());
    }
    let mut words = ryo_relations::domain::relations::types::Words::default();
    encode_table(&right, &[], &mut words);
    let ryo_values::domain::table_words::types::Admitted {
        table: compact,
        names,
    } = admit_compact(
        ryo_values::domain::table_words::types::CompactBacking(Arc::new(KeptWords(pack(
            WordSlice(&words.0),
        )))),
        &mut Vec::new(),
        DecodeMode::Complete,
    )
    .unwrap();
    assert!(names.is_empty());
    checked_refusal(
        &op,
        [Rows::Computed(&left), Rows::Compact(&compact)],
        &mut Symbols::default(),
    );

    let (elements, mut symbols) = wide_tree(1, true);
    let nodes = table_of(&[CellKind::Node], &[&[node(1)]]);
    let op = operation(&[0], &[0], &[(Side::Right, 0)], vec![], &[]);
    checked_refusal(
        &op,
        [
            Rows::Computed(&nodes),
            tree_rows(&elements, TreeForm::Elements),
        ],
        &mut symbols,
    );
    checked_refusal(
        &op,
        [
            tree_rows(&elements, TreeForm::Elements),
            Rows::Computed(&nodes),
        ],
        &mut symbols,
    );
}
fn empty_or_failed_left_never_enters_the_join_capability_or_builds_a_rhs_index() {
    let left = table_of(&[CellKind::Number], &[]);
    let right = table_of(
        &[CellKind::Number, CellKind::Number],
        &[&[Cell::Number(Integer(7)), Cell::Number(Integer(1))]],
    );
    let op = operation(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
    for failure in [None, Some(OperatingError::NotRun)] {
        for delivery in [false, true] {
            let witness = Arc::new(Witness {
                refuse: true,
                ..Witness::default()
            });
            let actual = invoke(
                &op,
                [Rows::Computed(&left), Rows::Computed(&right)],
                &mut Symbols::default(),
                b"",
                delivery,
                witness.clone(),
                Faults {
                    left: failure.clone(),
                    ..Faults::default()
                },
            )
            .unwrap();
            match &failure {
                Some(_) => assert!(matches!(actual.result, Err(OperatingError::NotRun))),
                None => assert!(actual.result.unwrap().cells.is_empty()),
            }
            assert_eq!(witness.calls.load(Ordering::Relaxed), 0);
            if let Some(error) = &failure {
                assert_eq!(actual.attempts, [SourceAttempt::Refused(0, error.clone())]);
                assert_eq!(actual.reads, [1, 0]);
            } else {
                assert_eq!(actual.attempts.first(), Some(&SourceAttempt::Available(0)));
                assert!(
                    actual
                        .attempts
                        .iter()
                        .all(|attempt| matches!(attempt, SourceAttempt::Available(_)))
                );
                assert!(
                    actual.attempts.contains(&SourceAttempt::Available(1)),
                    "canonical dependency admission still checks RHS after a successful empty LHS"
                );
            }
            assert_eq!(actual.work.index_rows, 0);
            assert_eq!(actual.work.index_cells, 0);
            assert!(actual.prefix.is_empty());
        }
    }
    let actual = invoke(
        &op,
        [Rows::Computed(&left), Rows::Computed(&right)],
        &mut Symbols::default(),
        b"",
        true,
        real_port(),
        Faults {
            right: Some(OperatingError::NotRead),
            ..Faults::default()
        },
    )
    .unwrap();
    assert!(matches!(actual.result, Err(OperatingError::NotRead)));
    assert_eq!(actual.work.index_rows, 0);
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    ComputedStoredCompactAndTreeProvidersReachTheSameRequiredCapability,
    EmptyOrFailedLeftNeverEntersTheJoinCapabilityOrBuildsARhsIndex,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "computed_stored_compact_and_tree_providers_reach_the_same_required_capability",
        ),
        word: Word::Succeeds,
        input: || Scenario::ComputedStoredCompactAndTreeProvidersReachTheSameRequiredCapability,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "empty_or_failed_left_never_enters_the_join_capability_or_builds_a_rhs_index",
        ),
        word: Word::Succeeds,
        input: || Scenario::EmptyOrFailedLeftNeverEntersTheJoinCapabilityOrBuildsARhsIndex,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::ComputedStoredCompactAndTreeProvidersReachTheSameRequiredCapability => {
            computed_stored_compact_and_tree_providers_reach_the_same_required_capability();
            Ok(())
        }
        Scenario::EmptyOrFailedLeftNeverEntersTheJoinCapabilityOrBuildsARhsIndex => {
            empty_or_failed_left_never_enters_the_join_capability_or_builds_a_rhs_index();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
