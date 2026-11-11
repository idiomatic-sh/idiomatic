//! Typed populations, dense bag order, partial provenance and fused reach semantics.
use super::super::super::super::*;
use super::super::cases::fixtures::{node, stretch, table_of};
use super::super::*;
use super::fixtures::functions;
use crate::cases::{Case, CaseName, Expect, Word, run};
#[cfg(test)]
use fixtures::execution::*;
#[cfg(test)]
use fixtures::provider::*;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::{
    service::kinds,
    types::{CellKind, FileId, Integer, Relation, Value as Cell},
};
use ryo_relations::domain::relations::{
    service::to_bits,
    types::{ColumnsView, FactColumn},
};
use ryo_values::domain::rows::service::row_in;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, ReachLookup, Side},
    plans::types::Operation,
    rows::types::RowCount,
};
fn composite_typed_keys_keep_duplicate_bag_order_under_stored_and_zero_budget_routes() {
    let left = functions(256, false);
    let columns = (0..left.kinds.len())
        .map(|column| {
            FactColumn(
                left.cells
                    .chunks_exact(4)
                    .map(|row| to_bits(row[column]).0)
                    .collect(),
            )
        })
        .collect::<Vec<_>>();
    let right = table_of(
        &[CellKind::Node, CellKind::File, CellKind::Number],
        &[
            &[node(3), Cell::File(FileId(0)), Cell::Number(Integer(30))],
            &[node(1), Cell::File(FileId(0)), Cell::Number(Integer(10))],
            &[node(3), Cell::File(FileId(0)), Cell::Number(Integer(31))],
            &[node(7), Cell::File(FileId(1)), Cell::Number(Integer(70))],
        ],
    );
    let op = operation(
        &[2, 0],
        &[1, 0],
        &[(Side::Left, 3), (Side::Right, 2), (Side::Left, 0)],
        vec![],
        &[],
    );
    let mut expected = vec![];
    for place in 0..256 {
        let at = place % 8 + 1;
        if at == 1 {
            expected.extend([
                stretch(place, place + 1),
                Cell::Number(Integer(10)),
                node(1),
            ]);
        }
        if at == 3 {
            for payload in [30, 31] {
                expected.extend([
                    stretch(place, place + 1),
                    Cell::Number(Integer(payload)),
                    node(3),
                ]);
            }
        }
    }
    for left in [
        Rows::Computed(&left),
        Rows::Stored(ColumnsView {
            kinds: kinds(Relation::Function),
            columns: &columns,
        }),
    ] {
        for optional_limit in [None, Some(0)] {
            let actual = invoke(
                &op,
                [left.clone(), Rows::Computed(&right)],
                &mut Symbols::default(),
                b"",
                false,
                real_port(),
                Faults {
                    optional_limit,
                    ..Faults::default()
                },
            )
            .unwrap();
            assert_eq!(actual.result.unwrap().cells, expected);
            assert!(actual.affected.is_empty());
        }
    }
}
fn dense_pairs_keep_all_65552_rows_and_first_last_order_without_optional_retention() {
    let left = functions(4097, true);
    let right = table_of(
        &[CellKind::Node, CellKind::Number],
        &(0..16)
            .map(|at| vec![node(1), Cell::Number(Integer(at))])
            .collect::<Vec<_>>()
            .iter()
            .map(Vec::as_slice)
            .collect::<Vec<_>>(),
    );
    let op = operation(
        &[0],
        &[0],
        &[(Side::Left, 3), (Side::Right, 1)],
        vec![],
        &[],
    );
    let actual = invoke(
        &op,
        [Rows::Computed(&left), Rows::Computed(&right)],
        &mut Symbols::default(),
        b"",
        false,
        real_port(),
        Faults {
            optional_limit: Some(0),
            ..Faults::default()
        },
    )
    .unwrap();
    let table = actual.result.unwrap();
    assert_eq!(table.count, RowCount(4097 * 16));
    assert_eq!(
        row_in(&table, ryo_values::domain::rows::types::RowIndex(0)),
        Some(&[stretch(0, 1), Cell::Number(Integer(0))][..])
    );
    assert_eq!(
        row_in(
            &table,
            ryo_values::domain::rows::types::RowIndex(table.count.0 - 1)
        ),
        Some(&[stretch(4096, 4097), Cell::Number(Integer(15))][..])
    );
    for (place, row) in table.cells.chunks_exact(2).enumerate() {
        assert_eq!(
            row,
            [
                stretch((place / 16) as u32, (place / 16 + 1) as u32),
                Cell::Number(Integer((place % 16) as i64))
            ]
        );
    }
}
fn partial_empty_population_keeps_exact_empty_rows_and_partial_provenance() {
    let left = functions(256, false);
    let right = table_of(&[CellKind::Node], &[&[node(99)]]);
    let actual = invoke(
        &operation(&[0], &[0], &[(Side::Left, 3)], vec![], &[]),
        [Rows::Computed(&left), Rows::Computed(&right)],
        &mut Symbols::default(),
        b"",
        false,
        real_port(),
        Faults {
            partials: vec![ryo_execution_domain::domain::coverage::types::Partial {
                relation: Relation::Entry,
                cause: ryo_values::domain::stored_facts::types::PartialCause(
                    "incomplete fixture".into(),
                ),
                lacking: ryo_execution_domain::domain::coverage::types::Lacking::Any,
                at: ryo_execution_domain::domain::coverage::types::PartialPlace::Repository,
            }],
            optional_limit: Some(0),
            ..Faults::default()
        },
    )
    .unwrap();
    assert!(actual.result.unwrap().cells.is_empty());
    assert_eq!(actual.affected, vec![Some(FileId(0))]);
}
fn reach_join_keeps_lazy_acquisition_exact_cycle_order_duplicates_and_refusals() {
    let n = |n| Cell::Number(Integer(n));
    let left = table_of(&[CellKind::Number], &[&[n(1)], &[n(1)]]);
    let empty = table_of(&[CellKind::Number], &[]);
    let right = table_of(
        &[CellKind::Number; 2],
        &[&[n(1), n(9)], &[n(1), n(2)], &[n(9), n(1)], &[n(2), n(3)]],
    );
    let mut op = operation(&[0], &[0], &[(Side::Right, 1)], vec![], &[]);
    let Operation::Join { reach, .. } = &mut op else {
        panic!("the fixture builds a Join")
    };
    *reach = Some(ReachLookup {
        source: ColumnIndex(0),
        destination: ColumnIndex(1),
        keys: vec![],
        columns: vec![ColumnIndex(0), ColumnIndex(1)],
    });
    let nothing = invoke(
        &op,
        [Rows::Computed(&empty), Rows::Computed(&right)],
        &mut Symbols::default(),
        b"",
        true,
        real_port(),
        Faults::default(),
    )
    .unwrap();
    assert!(nothing.result.unwrap().cells.is_empty());
    assert_eq!(nothing.counts.0, 0);
    assert!(
        nothing.derived_builds.is_empty(),
        "an empty primary builds neither partition order, closure adjacency nor an equality index"
    );
    assert!(
        nothing.attempts.contains(&SourceAttempt::Available(1)),
        "skipping payload never skips the required RHS source admission"
    );
    for (left_fault, right_fault, error) in [
        (None, Some(OperatingError::NotRead), OperatingError::NotRead),
        (Some(OperatingError::NotRun), None, OperatingError::NotRun),
    ] {
        let result = invoke(
            &op,
            [Rows::Computed(&left), Rows::Computed(&right)],
            &mut Symbols::default(),
            b"",
            true,
            real_port(),
            Faults {
                left: left_fault,
                right: right_fault,
                ..Faults::default()
            },
        )
        .unwrap();
        assert_eq!(
            format!("{:?}", result.result.unwrap_err()),
            format!("{error:?}")
        );
        assert!(result.prefix.is_empty());
    }
    let failed = invoke(
        &op,
        [Rows::Computed(&left), Rows::Computed(&right)],
        &mut Symbols::default(),
        b"",
        true,
        real_port(),
        Faults {
            sink_after: Some(0),
            ..Faults::default()
        },
    )
    .unwrap();
    assert!(matches!(failed.result, Err(OperatingError::NotRun)));
    let actual = invoke(
        &op,
        [Rows::Computed(&left), Rows::Computed(&right)],
        &mut Symbols::default(),
        b"",
        true,
        real_port(),
        Faults::default(),
    )
    .unwrap();
    assert_eq!(
        actual.prefix,
        [1, 2, 3, 9, 1, 2, 3, 9].map(|at| vec![n(at)])
    );
    assert_eq!(actual.work.traversal_queue_len, 0);
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    CompositeTypedKeysKeepDuplicateBagOrderUnderStoredAndZeroBudgetRoutes,
    DensePairsKeepAll65552RowsAndFirstLastOrderWithoutOptionalRetention,
    PartialEmptyPopulationKeepsExactEmptyRowsAndPartialProvenance,
    ReachJoinKeepsLazyAcquisitionExactCycleOrderDuplicatesAndRefusals,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(
            "composite_typed_keys_keep_duplicate_bag_order_under_stored_and_zero_budget_routes",
        ),
        word: Word::Succeeds,
        input: || Scenario::CompositeTypedKeysKeepDuplicateBagOrderUnderStoredAndZeroBudgetRoutes,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "dense_pairs_keep_all_65552_rows_and_first_last_order_without_optional_retention",
        ),
        word: Word::Succeeds,
        input: || Scenario::DensePairsKeepAll65552RowsAndFirstLastOrderWithoutOptionalRetention,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("partial_empty_population_keeps_exact_empty_rows_and_partial_provenance"),
        word: Word::Succeeds,
        input: || Scenario::PartialEmptyPopulationKeepsExactEmptyRowsAndPartialProvenance,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(
            "reach_join_keeps_lazy_acquisition_exact_cycle_order_duplicates_and_refusals",
        ),
        word: Word::Succeeds,
        input: || Scenario::ReachJoinKeepsLazyAcquisitionExactCycleOrderDuplicatesAndRefusals,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::CompositeTypedKeysKeepDuplicateBagOrderUnderStoredAndZeroBudgetRoutes => {
            composite_typed_keys_keep_duplicate_bag_order_under_stored_and_zero_budget_routes();
            Ok(())
        }
        Scenario::DensePairsKeepAll65552RowsAndFirstLastOrderWithoutOptionalRetention => {
            dense_pairs_keep_all_65552_rows_and_first_last_order_without_optional_retention();
            Ok(())
        }
        Scenario::PartialEmptyPopulationKeepsExactEmptyRowsAndPartialProvenance => {
            partial_empty_population_keeps_exact_empty_rows_and_partial_provenance();
            Ok(())
        }
        Scenario::ReachJoinKeepsLazyAcquisitionExactCycleOrderDuplicatesAndRefusals => {
            reach_join_keeps_lazy_acquisition_exact_cycle_order_duplicates_and_refusals();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
