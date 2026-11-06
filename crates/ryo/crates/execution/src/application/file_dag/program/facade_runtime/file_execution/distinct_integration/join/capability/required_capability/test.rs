//! Every provider kind reaches the required join capability, and empty or failed lefts never build a right index.
use super::super::super::cases::test::{node, table_of, tree_rows, wide_tree};
use super::super::super::*;
use super::super::fixtures::{Witness, refusal};
use std::sync::atomic::Ordering;
fn checked_refusal(op: &Operation, rows: [Rows<'_>; 2], symbols: &mut Symbols) {
    for delivery in [false, true] {
        for optional_limit in [None, Some(0)] {
            let witness = Arc::new(Witness {
                refuse: true,
                ..Witness::default()
            });
            let observed = invoke(
                op,
                rows.clone(),
                symbols,
                b"abcd",
                delivery,
                witness.clone(),
                Faults {
                    optional_limit,
                    ..Faults::default()
                },
            )
            .expect("real compiled owner");
            refusal(observed.result);
            assert!(observed.prefix.is_empty());
            assert_eq!(witness.calls.load(Ordering::Relaxed), 1);
            // A refusing dependency does not alter the immutable compiled meaning.
            let accepted = invoke(
                op,
                rows.clone(),
                symbols,
                b"abcd",
                delivery,
                real_port(),
                Faults {
                    optional_limit,
                    ..Faults::default()
                },
            )
            .unwrap();
            assert!(accepted.result.unwrap().count.0 > 0);
        }
    }
}
#[test]
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
    let (compact, names) = admit_compact(
        Arc::new(KeptWords(pack(WordSlice(&words.0)))),
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
#[test]
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
