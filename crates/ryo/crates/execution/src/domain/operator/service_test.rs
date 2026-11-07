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
use heck::ToSnakeCase;
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
    /// The engine reaches a kind's row meaning only through the one evaluation call: no execution source
    /// names a per-kind meaning of the operators.
    NoSecondImplementation,
    /// A driver port of operators is implemented by the engine's strategies only.
    DriversAreImplementedByStrategiesOnly,
    /// Every operator the documentation lists describes itself with a name, a summary and an example.
    EveryOperatorDescribesItself,
    /// Every kind's registered preparation refuses an operation of another kind.
    EveryKindRefusesAForeignOperation,
}
type Input = Scenario;
type Value = ();
type Failure = OperatingError;
/// The path prefixes, below the execution `src`, that hold an implementation of a driver port: the engine's
/// strategies behind the operators' ports (its join candidates, walks, graphs and delivery) and the
/// ordered-existence driver.
const STRATEGIES: &[&str] = &[
    "application/file_dag/ordered_existence.rs",
    "domain/operator/contract/",
];
/// The calls of the operators the engine may name: the one evaluation of every kind, the ordered-existence
/// driver of its operation, and the reductions that prepare a grouping resource.
const ENGINE_CALLS: &[&str] = &[
    "Evaluate",
    "OrderedExistence",
    "ReduceCount",
    "ReduceDigest",
];

fn crates() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("..")
}

fn sources(root: &Path, found: &mut Vec<PathBuf>) {
    let entries = std::fs::read_dir(root).expect("a directory of the census is readable");
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            sources(&path, found);
        } else if path.extension().is_some_and(|extension| extension == "rs") {
            found.push(path);
        }
    }
}

fn text(path: &Path) -> String {
    std::fs::read_to_string(path).expect("a source of the census is readable")
}

fn is_test(path: &Path) -> bool {
    let name = path.file_name().and_then(|name| name.to_str()).unwrap_or("");
    name.ends_with("_test.rs")
        || name == "test.rs"
        || name == "tests.rs"
        || path.components().any(|part| part.as_os_str() == "fixtures" || part.as_os_str() == "acceptance")
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

/// The census scenarios read the registry (`OperationKind::ALL`) and the sources of the two crates, never a
/// list kept by hand.
fn census(scenario: Scenario) {
    let execution = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
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
        Scenario::NoSecondImplementation => {
            let mut files = Vec::new();
            sources(&execution, &mut files);
            let kinds: Vec<String> = OperationKind::ALL
                .iter()
                .map(|kind| format!("{kind:?}").to_snake_case())
                .collect();
            let strays: Vec<String> = files
                .iter()
                .filter(|path| !is_test(path))
                .flat_map(|path| {
                    let source = text(path);
                    let mut found: Vec<String> = named_after(&source, "OperatorCall::")
                        .into_iter()
                        .filter(|call| !ENGINE_CALLS.contains(&call.as_str()))
                        .map(|call| format!("OperatorCall::{call} in {}", path.display()))
                        .collect();
                    let relative = path.strip_prefix(&execution).unwrap_or(path);
                    if relative.starts_with("domain/operator") {
                        found.extend(
                            relative
                                .components()
                                .filter_map(|part| part.as_os_str().to_str())
                                .map(|part| part.trim_end_matches(".rs"))
                                .filter(|part| kinds.iter().any(|kind| kind == part))
                                .map(|part| format!("module `{part}` in {}", path.display())),
                        );
                    }
                    found
                })
                .collect();
            assert!(
                strays.is_empty(),
                "a kind's meaning is implemented again in execution: {strays:?}"
            );
        }
        Scenario::DriversAreImplementedByStrategiesOnly => {
            let mut declared = Vec::new();
            sources(&crates().join("operators/src/domain"), &mut declared);
            let drivers: Vec<String> = declared
                .iter()
                .flat_map(|path| named_after(&text(path), "pub trait "))
                .filter(|name| name.ends_with("Driver"))
                .collect();
            assert!(!drivers.is_empty(), "operators declares no driver port");
            let mut files = Vec::new();
            sources(&execution, &mut files);
            let strays: Vec<String> = files
                .iter()
                .filter(|path| !is_test(path))
                .filter(|path| {
                    let relative = path.strip_prefix(&execution).unwrap_or(path).to_string_lossy();
                    !STRATEGIES.iter().any(|allowed| relative.starts_with(allowed))
                })
                .filter(|path| {
                    text(path).lines().any(|line| {
                        line.contains("impl")
                            && drivers.iter().any(|driver| line.contains(&format!("{driver} for ")))
                    })
                })
                .map(|path| path.display().to_string())
                .collect();
            assert!(strays.is_empty(), "driver ports implemented outside the strategies: {strays:?}");
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
            | Scenario::NoSecondImplementation
            | Scenario::DriversAreImplementedByStrategiesOnly
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
        | Scenario::NoSecondImplementation
        | Scenario::DriversAreImplementedByStrategiesOnly
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
        name: CaseName("no_operation_kind_is_implemented_again_in_execution"),
        word: Word::Succeeds,
        input: || Scenario::NoSecondImplementation,
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
        name: CaseName("driver_ports_are_implemented_by_strategies_only"),
        word: Word::Succeeds,
        input: || Scenario::DriversAreImplementedByStrategiesOnly,
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
