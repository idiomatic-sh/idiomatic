//! Preparation rejects foreign operations and retains input/value errors for execution.
use super::service::prepare_operator;
use crate::domain::{operating::errors::OperatingError, operation_schema::types::InputShape};
use ryo_relations::domain::relations::types::{CellKind, Relation};
use ryo_values::{
    cases::{Case, CaseName, Expect, Word, run},
    domain::{
        plan_parts::types::{ColumnIndex, Input as Source, Operand, RepeatedRows},
        plans::types::{Operation},
    },
};
use ryo_values::domain::query_operators::types::OperationKind;
use std::path::{Path, PathBuf};
#[derive(Clone, Copy)]
enum Scenario {
    Valid,
    Foreign,
    Missing,
    InvalidValue,
    /// Every registered kind has exactly one evaluation arm, in the operators' one evaluation.
    OneSemanticPerKind,
    /// Every operator the documentation lists describes itself with a name, a summary and an example.
    EveryOperatorDescribesItself,
    /// Every kind's registered preparation refuses an operation of another kind.
    EveryKindRefusesAForeignOperation,
}
type Input = Scenario;
type Value = ();
type Failure = OperatingError;
fn crates() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn text(path: &Path) -> String {
    std::fs::read_to_string(path).expect("a source of the census is readable")
}

/// The names after `marker` in `source`, each followed by a character that ends an identifier.
fn named_after(source: &str, marker: &str) -> Vec<String> {
    source
        .match_indices(marker)
        .map(|(at, _)| {
            source[at + marker.len()..]
                .chars()
                .take_while(|each| each.is_alphanumeric() || *each == '_')
                .collect::<String>()
        })
        .filter(|name| !name.is_empty())
        .collect()
}

/// The census scenarios read the registry (`OperationKind::ALL`) and the operators' one evaluation, never a
/// list kept by hand.
fn census(scenario: Scenario) {
    match scenario {
        Scenario::OneSemanticPerKind => {
            let dispatcher = text(&crates().join("operators/src/domain/evaluation/service.rs"));
            let arms = named_after(&dispatcher, "Operation::");
            assert!(!OperationKind::ALL.is_empty());
            let wrong: Vec<String> = OperationKind::ALL
                .iter()
                .map(|kind| format!("{kind:?}"))
                .filter(|kind| arms.iter().filter(|arm| *arm == kind).count() != 1)
                .collect();
            assert!(wrong.is_empty(), "kinds without exactly one evaluation in operators: {wrong:?}");
            let unknown: Vec<&String> = arms
                .iter()
                .filter(|arm| !OperationKind::ALL.iter().any(|kind| format!("{kind:?}") == **arm))
                .collect();
            assert!(unknown.is_empty(), "evaluations of no registered kind: {unknown:?}");
        }
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
                let foreign = if *kind == OperationKind::Union { &project } else { &union };
                assert_eq!(
                    prepare_operator(*kind, foreign, &[], &[]).map(|_| ()),
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
        Scenario::OneSemanticPerKind
            | Scenario::EveryOperatorDescribesItself
            | Scenario::EveryKindRefusesAForeignOperation
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
            &[InputShape { kinds: &kinds, tree: None }],
            &[],
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
        ),
        Scenario::Missing => prepare_operator(OperationKind::Project, &project, &[], &[]),
        Scenario::OneSemanticPerKind
        | Scenario::EveryOperatorDescribesItself
        | Scenario::EveryKindRefusesAForeignOperation => return Ok(()),
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
        name: CaseName("one_semantic_implementation_per_operation_kind"),
        word: Word::Succeeds,
        input: || Scenario::OneSemanticPerKind,
        expect: Expect::Value(()),
    },
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
