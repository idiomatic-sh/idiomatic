//! Direct catalog-relation Join routes retain their real acquired representation.
use super::super::*;
use super::fixtures::{encoded, request, schema_column, simple, table};
use crate::cases::{Case, CaseName, Expect, Word, run};
#[cfg(test)]
use fixtures::execution::*;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::table_model::types::RepresentationKind;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{CellKind, Flag, Integer, Relation, Value as Cell};
use ryo_relations::domain::relations::{service::from_bits, types::Bits};
use ryo_values::domain::{plan_parts::types::Side, rows::types::RowCount};

fn stored_and_compact_direct_join_composite_reordered_empty_and_unlike_keys_do_not_materialize_rhs()
{
    let n = |n| Cell::Number(Integer(n));
    let left = Relation::Entry;
    let right = Relation::SequenceElement;
    let ln = schema_column(left, CellKind::Number, 0);
    let lf = schema_column(left, CellKind::Flag, 0);
    let rn = schema_column(right, CellKind::Number, 0);
    let rf = schema_column(right, CellKind::Flag, 0);
    let rp = schema_column(right, CellKind::Number, 1);
    let lhs = table(
        left,
        &[(1, true), (1, false), (1, true), (9, true)].map(|(key, flag)| {
            vec![
                (ln, n(key)),
                (
                    lf,
                    Cell::Flag(if flag { Flag::Raised } else { Flag::Lowered }),
                ),
            ]
        }),
    );
    let rhs = table(
        right,
        &[(1, 2, 10), (2, 0, 20), (1, 1, 11), (3, 1, 30), (1, 0, 12)].map(
            |(key, flag, payload)| {
                vec![
                    (rn, n(key)),
                    (rf, from_bits(CellKind::Flag, Bits(flag))),
                    (rp, n(payload)),
                ]
            },
        ),
    );
    let output = [
        (Side::Right, rp),
        (Side::Left, ln),
        (Side::Right, rf),
        (Side::Right, rn),
        (Side::Right, rp),
    ];
    for (case, (l, r, expected_rows)) in [
        (vec![ln], vec![rn], 9),
        (vec![lf, ln], vec![rf, rn], 5),
        (vec![ln, lf], vec![rn, rf], 5),
        (vec![], vec![], 20),
        (vec![ln], vec![rf], 0),
    ]
    .into_iter()
    .enumerate()
    {
        let request = request(&l, &r, &output);
        let oracle = execute(
            request.clone(),
            [
                encoded(left, lhs.clone(), RepresentationKind::Computed),
                encoded(right, rhs.clone(), RepresentationKind::Computed),
            ],
            &mut Symbols::default(),
            b"",
            false,
            real_port(),
            Faults::default(),
        )
        .unwrap()
        .result
        .unwrap();
        assert_eq!(oracle.count, RowCount(expected_rows));
        let bag: Vec<(i64, i64, bool, i64)> = match case {
            0 => vec![
                (10, 1, true, 1),
                (11, 1, true, 1),
                (12, 1, false, 1),
                (10, 1, true, 1),
                (11, 1, true, 1),
                (12, 1, false, 1),
                (10, 1, true, 1),
                (11, 1, true, 1),
                (12, 1, false, 1),
            ],
            1 | 2 => vec![
                (10, 1, true, 1),
                (11, 1, true, 1),
                (12, 1, false, 1),
                (10, 1, true, 1),
                (11, 1, true, 1),
            ],
            3 => vec![
                (10, 1, true, 1),
                (20, 1, false, 2),
                (11, 1, true, 1),
                (30, 1, true, 3),
                (12, 1, false, 1),
                (10, 1, true, 1),
                (20, 1, false, 2),
                (11, 1, true, 1),
                (30, 1, true, 3),
                (12, 1, false, 1),
                (10, 1, true, 1),
                (20, 1, false, 2),
                (11, 1, true, 1),
                (30, 1, true, 3),
                (12, 1, false, 1),
                (10, 9, true, 1),
                (20, 9, false, 2),
                (11, 9, true, 1),
                (30, 9, true, 3),
                (12, 9, false, 1),
            ],
            4 => vec![],
            _ => panic!("the fixture lists every case it builds"),
        };
        let expected = bag
            .into_iter()
            .flat_map(|(payload, left, raised, right)| {
                [
                    n(payload),
                    n(left),
                    Cell::Flag(if raised { Flag::Raised } else { Flag::Lowered }),
                    n(right),
                    n(payload),
                ]
            })
            .collect::<Vec<_>>();
        assert_eq!(
            oracle.cells, expected,
            "independent ordered rows for case{case}"
        );
        for representation in [RepresentationKind::Stored, RepresentationKind::Compact] {
            for delivery in [false, true] {
                let actual = execute(
                    request.clone(),
                    [
                        encoded(left, lhs.clone(), RepresentationKind::Computed),
                        encoded(right, rhs.clone(), representation),
                    ],
                    &mut Symbols::default(),
                    b"",
                    delivery,
                    real_port(),
                    Faults::default(),
                )
                .unwrap();
                assert_eq!(actual.result.as_ref().unwrap().cells, oracle.cells);
                assert_eq!(
                    actual.work.index_representation,
                    Some(representation),
                    "the actual acquired Join index must retain its direct physical RHS representation"
                );
                assert_eq!(actual.work.index_cells, 0);
                assert_eq!(actual.work.index_capacity, 0);
                assert_eq!(actual.work.index_rows, 5);
            }
        }
    }
}
fn invalid_direct_join_keys_keep_exact_error_index_across_computed_stored_and_both_deliveries() {
    let l = schema_column(Relation::Entry, CellKind::Number, 0);
    let r = schema_column(Relation::SequenceElement, CellKind::Number, 0);
    let payload = schema_column(Relation::SequenceElement, CellKind::Number, 1);
    for (left, right) in [(vec![99], vec![r]), (vec![l], vec![99])] {
        let request = request(&left, &right, &[(Side::Right, payload)]);
        for delivery in [false, true] {
            let mut results = vec![];
            for representation in [RepresentationKind::Computed, RepresentationKind::Stored] {
                let actual = execute(
                    request.clone(),
                    simple(&[1], &[(1, 10)], representation),
                    &mut Symbols::default(),
                    b"",
                    delivery,
                    real_port(),
                    Faults::default(),
                )
                .unwrap();
                assert_eq!(
                    actual.result,
                    Err(OperatingError::Table(
                        ryo_values::domain::rows::errors::RowsError::NoSuchColumn { index: 99 }
                    ))
                );
                assert!(actual.prefix.is_empty());
                assert_eq!(actual.work.index_rows, 0);
                assert_eq!(actual.work.index_cells, 0);
                assert_eq!(actual.work.index_capacity, 0);
                assert_eq!(actual.work.index_representation, None);
                results.push(actual.result);
            }
            assert_eq!(results[0], results[1]);
        }
    }
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    StoredAndCompactDirectJoinCompositeReorderedEmptyAndUnlikeKeysDoNotMaterializeRhs,
    InvalidDirectJoinKeysKeepExactErrorIndexAcrossComputedStoredAndBothDeliveries,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "stored_and_compact_direct_join_composite_reordered_empty_and_unlike_keys_do_not_materialize_rhs",
        ),
        word: Word::Succeeds,
        input: || {
            Scenario::StoredAndCompactDirectJoinCompositeReorderedEmptyAndUnlikeKeysDoNotMaterializeRhs
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "invalid_direct_join_keys_keep_exact_error_index_across_computed_stored_and_both_deliveries",
        ),
        word: Word::Succeeds,
        input: || {
            Scenario::InvalidDirectJoinKeysKeepExactErrorIndexAcrossComputedStoredAndBothDeliveries
        },
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::StoredAndCompactDirectJoinCompositeReorderedEmptyAndUnlikeKeysDoNotMaterializeRhs => {
            stored_and_compact_direct_join_composite_reordered_empty_and_unlike_keys_do_not_materialize_rhs();
            Ok(())
        }
        Scenario::InvalidDirectJoinKeysKeepExactErrorIndexAcrossComputedStoredAndBothDeliveries => {
            invalid_direct_join_keys_keep_exact_error_index_across_computed_stored_and_both_deliveries();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
