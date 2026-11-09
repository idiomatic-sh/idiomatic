//! Preparation rejects foreign operations and retains input/value errors for execution.
use super::service::prepare_operator;

const PER_FILE: ryo_values::domain::plan_parts::types::StepScope =
    ryo_values::domain::plan_parts::types::StepScope::PerFile;
use crate::domain::{operating::errors::OperatingError, operation_schema::types::InputShape};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::domain::query_operators::types::OperationKind;
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{ColumnIndex, Input as Source, Operand, RepeatedRows},
        plans::types::Operation,
    },
};
#[derive(Clone, Copy)]
enum Scenario {
    Valid,
    Foreign,
    Missing,
    InvalidValue,
    /// Every operator the documentation lists describes itself with a name, a summary and an example.
    EveryOperatorDescribesItself,
    /// Every kind's registered preparation refuses an operation of another kind.
    EveryKindRefusesAForeignOperation,
}
type Input = Scenario;
type Value = ();
type Failure = OperatingError;
/// The registry scenarios read `OperationKind::ALL` and the documented operators, never a list kept by hand.
fn census(scenario: Scenario) {
    match scenario {
        Scenario::EveryOperatorDescribesItself => {
            for operator in super::constants::OPERATORS {
                let description = super::constants::describe(operator);
                assert!(
                    !description.name.0.is_empty()
                        && !description.summary.0.is_empty()
                        && description.example.0.contains("example"),
                    "{operator:?} does not describe itself"
                );
            }
        }
        Scenario::EveryKindRefusesAForeignOperation => {
            let union = Operation::Union {
                left: Source::Relation(Relation::Function),
                right: Source::Relation(Relation::Function),
            };
            let project = Operation::Project {
                from: Source::Relation(Relation::Function),
                columns: vec![ColumnIndex(0)],
                distinct: RepeatedRows::Dropped,
            };
            for kind in OperationKind::ALL {
                let foreign = if *kind == OperationKind::Union {
                    &project
                } else {
                    &union
                };
                assert_eq!(
                    prepare_operator(*kind, foreign, &[], &[], PER_FILE).map(|_| ()),
                    Err(OperatingError::WrongOperation),
                    "{kind:?} prepared an operation of another kind"
                );
            }
        }
        Scenario::Valid | Scenario::Foreign | Scenario::Missing | Scenario::InvalidValue => {}
    }
}
fn subject(scenario: Input) -> Result<Value, Failure> {
    if matches!(
        scenario,
        Scenario::EveryOperatorDescribesItself | Scenario::EveryKindRefusesAForeignOperation
    ) {
        census(scenario);
        return Ok(());
    }
    let project = Operation::Project {
        from: Source::Relation(Relation::Function),
        columns: vec![ColumnIndex(0)],
        distinct: RepeatedRows::Dropped,
    };
    let kinds = [CellKind::File];
    match scenario {
        Scenario::Valid => prepare_operator(
            OperationKind::Project,
            &project,
            &[InputShape {
                kinds: &kinds,
                tree: None,
            }],
            &[],
            PER_FILE,
        ),
        Scenario::Foreign => prepare_operator(
            OperationKind::Project,
            &Operation::Union {
                left: Source::Relation(Relation::Function),
                right: Source::Relation(Relation::Function),
            },
            &[InputShape {
                kinds: &kinds,
                tree: None,
            }],
            &[],
            PER_FILE,
        ),
        Scenario::Missing => prepare_operator(OperationKind::Project, &project, &[], &[], PER_FILE),
        Scenario::EveryOperatorDescribesItself | Scenario::EveryKindRefusesAForeignOperation => {
            return Ok(());
        }
        Scenario::InvalidValue => prepare_operator(
            OperationKind::Extend,
            &Operation::Extend {
                from: Source::Relation(Relation::Function),
                values: vec![Operand::Column(ColumnIndex(0))],
            },
            &[InputShape {
                kinds: &kinds,
                tree: None,
            }],
            &[],
            PER_FILE,
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
        name: CaseName("every_operator_describes_itself"),
        word: Word::Succeeds,
        input: || Scenario::EveryOperatorDescribesItself,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("every_kind_refuses_an_operation_of_another_kind"),
        word: Word::Succeeds,
        input: || Scenario::EveryKindRefusesAForeignOperation,
        expect: Expect::Value(()),
    },
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
