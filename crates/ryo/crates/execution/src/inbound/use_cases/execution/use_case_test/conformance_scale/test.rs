//! Actual engine visit and scratch bounds over increasing authored input populations.
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use super::super::conformance::test::{compare_delivery, repeated};
use crate::application::file_dag::acceptance::operator_contract_test::fixtures::*;
use crate::application::file_dag::acceptance::operator_contract_test::fixtures::values::numbers;
use crate::application::file_dag::acceptance::operator_contract_test::fixtures::expectations::SOURCE;
use ryo_language::domain::symbols::types::Symbols;
use ryo_values::domain::{
    plan_parts::types::ColumnIndex,
    plans::types::Operation,
    query_operators::types::OperationKind,
};
fn former_unary_pilots_keep_linear_visits_and_bounded_row_scratch() -> Result<(), Failure> {
    for kind in [
        OperationKind::Select,
        OperationKind::Project,
        OperationKind::SourceMatches,
        OperationKind::SpanBounds,
    ] {
        let mut case = fixture(kind);
        if let Operation::Project { distinct, .. } = &mut case.operation {
            *distinct = ryo_values::domain::plan_parts::types::RepeatedRows::Kept;
        }
        let small = vec![repeated(&case.inputs[0], 32)];
        let large = vec![repeated(&case.inputs[0], 64)];
        let first = compare_delivery(
            &case.operation,
            &small,
            &[],
            SOURCE,
            &mut case.symbols,
            None,
        )
        .unwrap();
        let second = compare_delivery(
            &case.operation,
            &large,
            &[],
            SOURCE,
            &mut case.symbols,
            None,
        )
        .unwrap();
        assert!(first.visits[0] > 0);
        assert_eq!(second.visits[0], first.visits[0] * 2);
        assert_eq!(second.rows.len(), first.rows.len() * 2);
        assert_eq!(second.scratch_cells, first.scratch_cells);
        let empty = compare_delivery(
            &case.operation,
            &[repeated(&case.inputs[0], 0)],
            &[],
            SOURCE,
            &mut case.symbols,
            None,
        )
        .unwrap();
        assert!(empty.rows.is_empty());
        assert_eq!(empty.visits, vec![0]);
    }
    Ok(())
}
fn former_binary_pilots_keep_linear_visits_for_unique_keys() -> Result<(), Failure> {
    for kind in [OperationKind::Join, OperationKind::Union] {
        let mut case = fixture(kind);
        if let Operation::Join { output, .. } = &mut case.operation {
            for column in output {
                column.column = ColumnIndex(0);
            }
        }
        let run = |count: i64, symbols: &mut Symbols| {
            let values = (0..count).collect::<Vec<_>>();
            compare_delivery(
                &case.operation,
                &[numbers(&values), numbers(&values)],
                &[],
                SOURCE,
                symbols,
                None,
            )
            .unwrap()
        };
        let first = run(32, &mut case.symbols);
        let second = run(64, &mut case.symbols);
        assert_eq!((first.rows.len(), second.rows.len()), (32, 64));
        for side in 0..2 {
            assert!(first.visits[side] > 0);
            assert_eq!(second.visits[side], first.visits[side] * 2);
        }
        let empty = run(0, &mut case.symbols);
        assert!(empty.rows.is_empty());
        assert_eq!(empty.visits, vec![0, 0]);
    }
    Ok(())
}
/// The scenarios of this group, one row each.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    FormerUnaryPilotsKeepLinearVisitsAndBoundedRowScratch,
    FormerBinaryPilotsKeepLinearVisitsForUniqueKeys,
}
type Input = Scenario;
type Value = ();
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName(stringify!(former_unary_pilots_keep_linear_visits_and_bounded_row_scratch)),
        word: Word::Succeeds,
        input: || Scenario::FormerUnaryPilotsKeepLinearVisitsAndBoundedRowScratch,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName(stringify!(former_binary_pilots_keep_linear_visits_for_unique_keys)),
        word: Word::Succeeds,
        input: || Scenario::FormerBinaryPilotsKeepLinearVisitsForUniqueKeys,
        expect: Expect::Value(()),
    },
];
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::FormerUnaryPilotsKeepLinearVisitsAndBoundedRowScratch => former_unary_pilots_keep_linear_visits_and_bounded_row_scratch(),
        Scenario::FormerBinaryPilotsKeepLinearVisitsForUniqueKeys => former_binary_pilots_keep_linear_visits_for_unique_keys(),
    }
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
