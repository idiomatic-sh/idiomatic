//! Actual invocation storage measurements through normal compiled file execution.
use super::fixtures::{execute, hosted};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_language::domain::symbols::types::Symbols;
use ryo_relations::domain::relations::types::{Integer, Value as Cell};
use ryo_values::domain::{
    hosted::types::HostKind,
    plan_parts::types::{Operand, PlanComparison, Test},
    row_texts::types::Cell as HostedCell,
};
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput, RepeatedRows, StepId},
    plans::types::Operation,
};
fn projection_constructs_only_the_group_it_uses_and_keeps_bags() {
    for (distinct, expected, groups) in [
        (RepeatedRows::Kept, vec![2, 1, 2], 0),
        (RepeatedRows::Dropped, vec![2, 1], 1),
    ] {
        let host = hosted(
            &[HostKind::Number],
            [2, 1, 2]
                .map(|number| vec![HostedCell::Number(Integer(number))])
                .into(),
        );
        let (result, observed) = execute(
            Operation::Project {
                from: PlanInput::Step(StepId(0)),
                columns: vec![ColumnIndex(0)],
                distinct,
            },
            host,
            &mut Symbols::default(),
        );
        assert_eq!(
            result.unwrap().cells,
            expected
                .into_iter()
                .map(|number| Cell::Number(Integer(number)))
                .collect::<Vec<_>>()
        );
        assert_eq!(observed.owners_initialized, groups);
    }
}
fn dynamic_regex_initialization_is_execution_local_and_lazy() {
    for populated in [false, true, false] {
        let rows = if populated {
            vec![vec![HostedCell::Text("[".into())]]
        } else {
            vec![]
        };
        let (result, observed) = execute(
            Operation::Select {
                from: PlanInput::Step(StepId(0)),
                comparisons: vec![PlanComparison {
                    test: Test::Matches,
                    left: Operand::Column(ColumnIndex(0)),
                    right: Operand::Column(ColumnIndex(0)),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
                kept: vec![ColumnIndex(0)],
            },
            hosted(&[HostKind::Text], rows),
            &mut Symbols::default(),
        );
        if populated {
            assert!(matches!(result, Err(OperatingError::InvalidRegex { .. })));
        } else {
            assert_eq!(result.unwrap().count.0, 0);
        }
        assert_eq!(
            observed.regexes, 0,
            "invalid dynamic patterns are never cached"
        );
    }
}
fn aggregation_resources_are_engine_owned_and_released() {
    use crate::application::file_dag::fixtures::operator_contract::{
        expectations::assert_expected, fixture,
    };
    use ryo_values::domain::query_operators::types::OperationKind;
    for kind in [OperationKind::GroupCount, OperationKind::GroupDigest] {
        let mut case = fixture(kind);
        let input = &case.inputs[0];
        let rows = input
            .cells
            .chunks_exact(input.kinds.len())
            .map(|row| {
                row.iter()
                    .map(|value| match value {
                        Cell::Number(number) => HostedCell::Number(*number),
                        other => panic!("numeric aggregation fixture, got {other:?}"),
                    })
                    .collect()
            })
            .collect();
        let operation = match case.operation {
            Operation::GroupCount { keys, distinct, .. } => Operation::GroupCount {
                from: PlanInput::Step(StepId(0)),
                keys,
                distinct,
            },
            Operation::GroupDigest { keys, over, .. } => Operation::GroupDigest {
                from: PlanInput::Step(StepId(0)),
                keys,
                over,
            },
            _ => panic!("the fixture lists every case it builds"),
        };
        let (result, observed) = execute(
            operation,
            hosted(&vec![HostKind::Number; input.kinds.len()], rows),
            &mut case.symbols,
        );
        let out = result.unwrap();
        let rows = out
            .cells
            .chunks_exact(out.kinds.len())
            .map(<[_]>::to_vec)
            .collect::<Vec<_>>();
        assert_expected(&case.expected, &rows, &case.symbols);
        assert_eq!(observed.owners_initialized, 1, "{kind:?}");
    }
}

/// The case each row runs: one of the proofs below.
#[derive(Clone, Copy, Debug)]
enum Scenario {
    ProjectionConstructsOnlyTheGroupItUsesAndKeepsBags,
    DynamicRegexInitializationIsExecutionLocalAndLazy,
    AggregationResourcesAreEngineOwnedAndReleased,
}

type Input = Scenario;
type Value = ();
type Failure = String;

/// One row per proof: its name, and the scenario that runs it.
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("projection_constructs_only_the_group_it_uses_and_keeps_bags"),
        word: Word::Succeeds,
        input: || Scenario::ProjectionConstructsOnlyTheGroupItUsesAndKeepsBags,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("dynamic_regex_initialization_is_execution_local_and_lazy"),
        word: Word::Succeeds,
        input: || Scenario::DynamicRegexInitializationIsExecutionLocalAndLazy,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("aggregation_resources_are_engine_owned_and_released"),
        word: Word::Succeeds,
        input: || Scenario::AggregationResourcesAreEngineOwnedAndReleased,
        expect: Expect::Value(()),
    },
];

/// Runs one scenario; its assertions are the case's expectation.
fn subject(input: Input) -> Result<Value, Failure> {
    match input {
        Scenario::ProjectionConstructsOnlyTheGroupItUsesAndKeepsBags => {
            projection_constructs_only_the_group_it_uses_and_keeps_bags();
            Ok(())
        }
        Scenario::DynamicRegexInitializationIsExecutionLocalAndLazy => {
            dynamic_regex_initialization_is_execution_local_and_lazy();
            Ok(())
        }
        Scenario::AggregationResourcesAreEngineOwnedAndReleased => {
            aggregation_resources_are_engine_owned_and_released();
            Ok(())
        }
    }
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
