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
    /// Every registered kind has exactly one item directory, and every item directory is a registered kind.
    KindsAndItemsAgree,
    /// No free function an operators service defines is defined again by an operator item of execution.
    NoSemanticIsDefinedTwice,
    /// A driver port of operators is implemented by the strategies of the route policy and the generic items only.
    DriversAreImplementedByStrategiesOnly,
}
type Input = Scenario;
type Value = ();
type Failure = OperatingError;
/// The path prefixes, below the execution `src`, that hold an implementation of a driver port: one for each strategy of
/// `RoutePolicy`, and the generic evaluation of the operators.
const STRATEGIES: &[&str] = &[
    "application/file_dag/ordered_existence.rs",
    "domain/column_kernel/",
    "domain/native_positions/",
    "domain/tree_join/",
    "domain/operator/collections/operators/",
    "domain/operator/contract/set_rows.rs",
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
    name.ends_with("_test.rs") || name == "test.rs" || name == "tests.rs"
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

/// The names of the functions declared at the left margin: methods of an impl are indented and never counted.
fn free_functions(source: &str) -> Vec<String> {
    source
        .lines()
        .filter_map(|line| {
            let rest = line
                .strip_prefix("pub(crate) ")
                .or_else(|| line.strip_prefix("pub(super) "))
                .or_else(|| line.strip_prefix("pub "))
                .unwrap_or(line);
            rest.strip_prefix("fn ")
        })
        .map(|rest| {
            rest.chars()
                .take_while(|each| each.is_alphanumeric() || *each == '_')
                .collect()
        })
        .collect()
}

/// The census scenarios read the registry (`OperationKind::ALL`) and the sources of the two crates, never a list kept by hand.
fn census(scenario: Scenario) {
    let execution = Path::new(env!("CARGO_MANIFEST_DIR")).join("src");
    let items = execution.join("domain/operator/collections/operators");
    match scenario {
        Scenario::KindsAndItemsAgree => {
            let mut kinds: Vec<String> = OperationKind::ALL
                .iter()
                .map(|kind| format!("{kind:?}").to_snake_case())
                .collect();
            let mut directories: Vec<String> = std::fs::read_dir(&items)
                .expect("the items directory is readable")
                .flatten()
                .filter(|entry| entry.path().join("item.rs").exists())
                .filter_map(|entry| entry.file_name().into_string().ok())
                .collect();
            kinds.sort();
            directories.sort();
            assert!(!kinds.is_empty());
            assert_eq!(kinds, directories, "kinds differ from item directories");
        }
        Scenario::NoSemanticIsDefinedTwice => {
            let mut services = Vec::new();
            sources(&crates().join("operators/src/domain"), &mut services);
            let defined: std::collections::BTreeSet<String> = services
                .iter()
                .filter(|path| path.file_name().is_some_and(|name| name == "service.rs"))
                .flat_map(|path| named_after(&text(path), "fn "))
                .collect();
            assert!(!defined.is_empty(), "operators defines no service function");
            let mut files = Vec::new();
            sources(&items, &mut files);
            let twice: Vec<String> = files
                .iter()
                .filter(|path| !is_test(path) && !path.to_string_lossy().contains("/fixtures"))
                .flat_map(|path| {
                    free_functions(&text(path))
                        .into_iter()
                        .filter(|name| defined.contains(name))
                        .map(|name| format!("{name} in {}", path.display()))
                        .collect::<Vec<_>>()
                })
                .collect();
            assert!(twice.is_empty(), "defined in operators and again in execution: {twice:?}");
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
        Scenario::Valid | Scenario::Foreign | Scenario::Missing | Scenario::InvalidValue => {}
    }
}
fn subject(scenario: Input) -> Result<Value, Failure> {
    if matches!(
        scenario,
        Scenario::KindsAndItemsAgree
            | Scenario::NoSemanticIsDefinedTwice
            | Scenario::DriversAreImplementedByStrategiesOnly
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
        Scenario::KindsAndItemsAgree
        | Scenario::NoSemanticIsDefinedTwice
        | Scenario::DriversAreImplementedByStrategiesOnly => return Ok(()),
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
        name: CaseName("every_operation_kind_has_exactly_one_item"),
        word: Word::Succeeds,
        input: || Scenario::KindsAndItemsAgree,
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("no_operators_function_is_defined_again_in_execution"),
        word: Word::Succeeds,
        input: || Scenario::NoSemanticIsDefinedTwice,
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
