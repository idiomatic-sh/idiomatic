//! Binding identity and file generations through the actual compiled request API.
use crate::application::file_dag::{
    execution::CommonOperatorEngine,
    program::{
        facade::{self, EmitDemand, ObservationDemand, RootOutcome, RuleDemand},
        facade_runtime::{FileRun, ProgramServices, open_file},
    },
    scope::ScopePartials,
};
use crate::domain::{
    coverage::types::RuleIndex,
    file_program::types::RuleSchedule,
    operating::errors::OperatingError,
    prepared_request::types::{Compiled, ObservationIdentity},
    table_model::types::Rows,
};
use ryo_language::domain::{files::types::FilePath, symbols::types::Symbols};
use ryo_relations::domain::relations::{
    service::{kinds, to_bits},
    types::{ColumnsView, FactColumn, FileId, Integer, Relation, Value},
};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{
        ColumnIndex, Comparison, Input, Operand, ParameterId, ParameterName, RepeatedRows, StepId,
        Test, WholeNumber,
    },
    plans::types::{Emit, Operation, ParameterType, Plan, PlanParameter, PlanStep, StepScope},
    program::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
    query_bindings::types::ParameterBinding,
    rows::types::Table,
    rule_requests::types::{RuleParameters, RuleSpec},
    runs::types::{RuleApplicability, RuleId},
    stored_facts::types::Digest,
};
use std::{collections::BTreeMap, sync::Arc};
fn binding_plan() -> Plan {
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![PlanParameter {
            name: ParameterName("major".into()),
            kind: ParameterType::Number,
        }],
        steps: vec![
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Select {
                    from: Input::Relation(Relation::LanguageTarget),
                    comparisons: vec![Comparison {
                        test: Test::Equal,
                        left: Operand::Column(ColumnIndex(1)),
                        right: Operand::Parameter(ParameterId(0)),
                        origin: None,
                    }],
                    kept: vec![],
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: Input::Step(StepId(0)),
                    columns: vec![ColumnIndex(0)],
                    distinct: RepeatedRows::Kept,
                },
            },
            PlanStep {
                scope: StepScope::PerFile,
                operation: Operation::Project {
                    from: Input::Step(StepId(1)),
                    columns: vec![ColumnIndex(0)],
                    distinct: RepeatedRows::Kept,
                },
            },
        ],
        emit: Emit {
            step: StepId(2),
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
        frontier: vec![],
        comparison_origins: vec![],
    }
}
fn rule(index: usize) -> RuleSpec {
    RuleSpec {
        id: RuleId(format!("generation-{index}").into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId(format!("generation-{index}").into()),
                text: "".into(),
            },
            modules: BTreeMap::new(),
        },
        applicability: RuleApplicability::Always,
        given: vec![],
        observations: vec![],
        spelled: RuleParameters("".into()),
        include: vec![],
        exclude: vec![],
    }
}
fn prepare(
    bindings: Vec<Vec<ParameterBinding>>,
) -> (Arc<facade::PreparedProgram>, Arc<ProgramServices>) {
    let count = bindings.len();
    let derived = crate::application::file_dag::testing::program_derived_port();
    let prepared = facade::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]; count],
            plans: vec![binding_plan(); count],
            bindings,
        },
        &(0..count).map(rule).collect::<Vec<_>>(),
        &vec![Digest([0; 16]); count],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )
    .unwrap();
    let services = Arc::new(ProgramServices::new(
        Arc::new(CommonOperatorEngine::new(std::sync::Arc::new(
            ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
        ))),
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("generation.py".into()),
            size: FileSize(0),
            modified: Nanoseconds(0),
        })],
    ));
    (prepared, services)
}
fn language_columns(file: u32, major: i64) -> Vec<FactColumn> {
    [
        Value::File(FileId(file)),
        Value::Number(Integer(major)),
        Value::Number(Integer(10)),
    ]
    .into_iter()
    .map(|cell| FactColumn(vec![to_bits(cell).0]))
    .collect()
}
fn run(
    file: &mut FileRun<'_>,
    rule: usize,
    storage: &[FactColumn],
    symbols: &mut Symbols,
) -> Table {
    let reads = std::cell::Cell::new(0);
    let read = |relation| {
        reads.set(reads.get() + 1);
        if relation == Relation::LanguageTarget {
            Ok(Rows::Stored(ColumnsView {
                kinds: kinds(relation),
                columns: storage,
            }))
        } else {
            Err(OperatingError::NotRead)
        }
    };
    let outcome = file
        .rule(
            RuleIndex(rule),
            RuleDemand {
                emit: EmitDemand::Enabled,
                observations: ObservationDemand::NotRequired,
            },
            &read,
            symbols,
        )
        .unwrap();
    assert!(
        reads.get() > 0,
        "each actual generation consults its own provider"
    );
    let RootOutcome::Complete(table) = outcome.emit else {
        panic!("compiled file rule must complete")
    };
    table
}
#[test]
fn identical_binding_dependent_operations_do_not_share_rule_graph_results() {
    let (prepared, services) = prepare(vec![
        vec![ParameterBinding::Number(WholeNumber(3))],
        vec![ParameterBinding::Number(WholeNumber(2))],
    ]);
    let program = prepared.program();
    let first = program.rules()[0].original_steps[0]
        .as_ref()
        .unwrap()
        .logical;
    let second = program.rules()[1].original_steps[0]
        .as_ref()
        .unwrap()
        .logical;
    assert_ne!(
        program.logical_regions()[first.0],
        program.logical_regions()[second.0]
    );
    let mut symbols = Symbols::default();
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[true, true],
        services,
        b"",
        true,
        &mut symbols,
        None,
        None,
    )
    .unwrap();
    let storage = language_columns(7, 3);
    assert_eq!(
        run(&mut file, 0, &storage, &mut symbols).cells,
        vec![Value::File(FileId(7))]
    );
    assert_eq!(run(&mut file, 1, &storage, &mut symbols).count.0, 0);
    file.finish().unwrap();
}
#[test]
fn two_file_generation_owners_cannot_reuse_the_other_files_results() {
    let (prepared, services) = prepare(vec![vec![ParameterBinding::Number(WholeNumber(3))]]);
    let mut first_symbols = Symbols::default();
    let mut first = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[true],
        Arc::clone(&services),
        b"",
        true,
        &mut first_symbols,
        None,
        None,
    )
    .unwrap();
    let left = run(&mut first, 0, &language_columns(1, 3), &mut first_symbols);
    let mut second_symbols = Symbols::default();
    let mut second = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[true],
        services,
        b"",
        true,
        &mut second_symbols,
        None,
        None,
    )
    .unwrap();
    let right = run(&mut second, 0, &language_columns(9, 3), &mut second_symbols);
    assert_eq!(left.cells, vec![Value::File(FileId(1))]);
    assert_eq!(right.cells, vec![Value::File(FileId(9))]);
    first.finish().unwrap();
    second.finish().unwrap();
    assert_eq!(
        left.cells,
        vec![Value::File(FileId(1))],
        "transferred first output remains independent"
    );
}
