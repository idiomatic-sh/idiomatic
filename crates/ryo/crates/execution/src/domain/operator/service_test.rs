//! Preparation rejects foreign operations and retains input/value errors for execution.
use super::service::registered;
use crate::domain::{operating::errors::OperatingError, operation_schema::types::InputShape};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{ColumnIndex, Input as Source, Operand, RepeatedRows},
        plans::types::{Operation, OperationKind},
    },
};
#[derive(Clone, Copy)]
enum Scenario {
    Valid,
    Foreign,
    Missing,
    InvalidValue,
}
type Input = Scenario;
type Value = ();
type Failure = OperatingError;
fn subject(scenario: Input) -> Result<Value, Failure> {
    let project = Operation::Project {
        from: Source::Relation(Relation::Function),
        columns: vec![ColumnIndex(0)],
        distinct: RepeatedRows::Dropped,
    };
    let kinds = [CellKind::File];
    match scenario {
        Scenario::Valid => registered(OperationKind::Project).prepare(
            &project,
            &[InputShape { kinds: &kinds, tree: None }],
            &[],
        ),
        Scenario::Foreign => registered(OperationKind::Project).prepare(
            &Operation::Union {
                left: Source::Relation(Relation::Function),
                right: Source::Relation(Relation::Function),
            },
            &[InputShape {
                kinds: &kinds,
                tree: None,
            }],
            &[],
        ),
        Scenario::Missing => registered(OperationKind::Project).prepare(&project, &[], &[]),
        Scenario::InvalidValue => registered(OperationKind::Extend).prepare(
            &Operation::Extend {
                from: Source::Relation(Relation::Function),
                values: vec![Operand::Column(ColumnIndex(0))],
            },
            &[InputShape {
                kinds: &kinds,
                tree: None,
            }],
            &[],
        ),
    }
    .and_then(|contract| {
        if matches!(scenario, Scenario::Valid) {
            assert_eq!(contract.schema, Ok(vec![CellKind::File]));
            assert_eq!(contract.output_kinds, Ok(vec![CellKind::File]));
        }
        contract.schema.map(|_| ())
    })
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("a_valid_projection_retains_its_output_schema"),
        word: Word::Succeeds,
        input: || Scenario::Valid,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("an_operator_refuses_an_operation_that_is_not_its_own"),
        word: Word::Refuses,
        input: || Scenario::Foreign,
        expect: Expect::Error(OperatingError::WrongOperation),
    },
    Case {
        name: CaseName("an_operator_requires_its_declared_input"),
        word: Word::Refuses,
        input: || Scenario::Missing,
        expect: Expect::Error(OperatingError::MissingInput { position: 1 }),
    },
    Case {
        name: CaseName("a_column_is_not_a_value_to_append"),
        word: Word::Refuses,
        input: || Scenario::InvalidValue,
        expect: Expect::Error(OperatingError::WrongBinding),
    },
];
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
