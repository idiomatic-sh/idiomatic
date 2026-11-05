//! Select, count and refusal cases through the prepared file program.

use crate::application::{
    file_dag::{
        execution::CommonOperatorEngine,
        program::{
            facade::{self, EmitDemand, ObservationDemand, RootOutcome, RuleDemand},
            facade_runtime::{ProgramServices, open_file},
        },
        scope::ScopePartials,
    },
    rule_compilation::types::{Compiled, ObservationIdentity},
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    file_program::types::RuleSchedule,
    operating::errors::OperatingError,
    positions::service_test::{SMALL_SOURCE, SmallTree, rows_over, small_tree},
    rules::types::{RuleParameters, RuleSpec},
    table_model::types::Rows,
};
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::{
    service::{kinds, node_id},
    types::{CellKind, FileId, Flag, Integer, Offset, Position, Relation, Span, Value as Cell},
};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{
        ColumnIndex, Comparison, Input as PlanInput, JoinColumn, LiteralText, Operand,
        QueryLiteral, Side, StepId, Test,
    },
    plans::types::{Emit, Operation, Plan, PlanStep, StepScope},
    program::types::{QueryBundle, QueryDocumentSource, QuerySourceId},
    rows::types::{RowCount, Table},
    runs::types::{RuleApplicability, RuleId},
    stored_facts::types::Digest,
};
use std::{collections::BTreeMap, sync::Arc};

#[derive(Clone, Copy)]
enum Scenario {
    Words,
    Counted,
    MissingRight,
    OrdinaryFunctionJoin,
}
type Input = Scenario;
type Value = (usize, &'static str);
type Failure = OperatingError;

fn plan(scenario: Scenario) -> Plan {
    let mut steps = vec![PlanStep {
        operation: Operation::Select {
            from: PlanInput::Relation(Relation::SyntaxTree),
            comparisons: if matches!(scenario, Scenario::OrdinaryFunctionJoin) {
                vec![]
            } else {
                vec![Comparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(1)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("word".into()))),
                    origin: None,
                }]
            },
            kept: vec![ColumnIndex(0), ColumnIndex(1)],
        },
        scope: StepScope::PerFile,
    }];
    if matches!(scenario, Scenario::Counted) {
        steps.push(PlanStep {
            operation: Operation::GroupCount {
                from: PlanInput::Step(StepId(0)),
                keys: vec![ColumnIndex(1)],
                distinct: None,
            },
            scope: StepScope::PerFile,
        });
    }
    if matches!(scenario, Scenario::MissingRight) {
        steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Select {
                from: PlanInput::Relation(Relation::SyntaxChild),
                comparisons: vec![],
                kept: vec![],
            },
        });
        steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Antijoin {
                left: PlanInput::Step(StepId(0)),
                right: PlanInput::Step(StepId(1)),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(1)],
            },
        });
    }
    if matches!(scenario, Scenario::OrdinaryFunctionJoin) {
        steps.push(PlanStep {
            scope: StepScope::PerFile,
            operation: Operation::Join {
                left: PlanInput::Step(StepId(0)),
                right: PlanInput::Relation(Relation::Function),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
                output: vec![JoinColumn {
                    side: Side::Left,
                    column: ColumnIndex(0),
                }],
                filter: vec![],
                view: vec![],
                reach: None,
            },
        });
    }
    let last = StepId((steps.len() - 1) as u32);
    Plan {
        observation_inputs: vec![],
        observations: vec![],
        parameters: vec![],
        steps,
        comparison_origins: vec![],
        frontier: vec![],
        emit: Emit {
            step: last,
            subject: ColumnIndex(0),
            key: vec![],
            headings: vec![],
        },
    }
}
fn rule() -> RuleSpec {
    RuleSpec {
        id: RuleId("native-parity".into()),
        query: QueryBundle {
            root: QueryDocumentSource {
                id: QuerySourceId("native-parity".into()),
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
fn subject(scenario: Input) -> Result<Value, Failure> {
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let derived = crate::application::file_dag::testing::program_derived_port();
    let prepared = facade::prepare(
        Compiled {
            schedule: RuleSchedule::Independent,
            observations: vec![vec![]],
            plans: vec![plan(scenario)],
            bindings: vec![vec![]],
        },
        &[rule()],
        &[Digest([0; 16])],
        &[],
        Arc::clone(&derived),
        ObservationIdentity::Independent,
    )?;
    let engine = CommonOperatorEngine::new(crate::outbound::operators::factory::test_port());
    let services = ProgramServices::new(
        &engine,
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("native-parity.py".into()),
            size: FileSize(SMALL_SOURCE.len() as u64),
            modified: Nanoseconds(0),
        })],
    );
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[true],
        &services,
        SMALL_SOURCE,
        &mut symbols,
        None,
        None,
    )?;
    let function = Table {
        kinds: kinds(Relation::Function).to_vec(),
        cells: kinds(Relation::Function)
            .iter()
            .copied()
            .map(|kind| match kind {
                CellKind::Node => Cell::Node(node_id(FileId(0), Position(2))),
                CellKind::Symbol => Cell::Symbol(ryo_relations::domain::terms::types::Symbol(0)),
                CellKind::File => Cell::File(FileId(0)),
                CellKind::Span => Cell::Span(Span {
                    start: Offset(0),
                    end: Offset(0),
                }),
                CellKind::Number => Cell::Number(Integer(0)),
                CellKind::Flag => Cell::Flag(Flag::Lowered),
            })
            .collect(),
        count: RowCount(1),
    };
    let read = |relation| match relation {
        Relation::SyntaxTree => Ok(Rows::Tree(rows_over(&elements))),
        Relation::Function => Ok(Rows::Computed(&function)),
        _ => Err(OperatingError::NotRead),
    };
    let outcome = file.rule(
        crate::domain::coverage::types::RuleIndex(0),
        RuleDemand {
            emit: EmitDemand::Enabled,
            observations: ObservationDemand::NotRequired,
        },
        &read,
        &mut symbols,
    )?;
    let table = match outcome.emit {
        RootOutcome::Complete(table) => table,
        RootOutcome::Unknown(error) | RootOutcome::Failed(error) => return Err(error),
        _ => return Err(OperatingError::NotRun),
    };
    let shown = table
        .cells
        .iter()
        .map(|cell| match cell {
            Cell::Node(node) => node.0.to_string(),
            Cell::Number(Integer(number)) => number.to_string(),
            Cell::Symbol(_) | Cell::Name(_) | Cell::File(_) | Cell::Span(_) | Cell::Flag(_) => {
                "_".to_owned()
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    Ok((table.count.0, Box::leak(shown.into_boxed_str())))
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("the_rows_of_a_lowered_select_are_passed_on_as_values"),
        word: Word::Succeeds,
        input: || Scenario::Words,
        expect: Expect::Value((2, "1 _ 3 _")),
    },
    Case {
        name: CaseName("a_count_of_lowered_rows_is_passed_on_with_the_keys"),
        word: Word::Succeeds,
        input: || Scenario::Counted,
        expect: Expect::Value((1, "_ 2")),
    },
    Case {
        name: CaseName("non_tree_join_uses_registered_operator_after_native_decline"),
        word: Word::Succeeds,
        input: || Scenario::OrdinaryFunctionJoin,
        expect: Expect::Value((1, "2")),
    },
    Case {
        name: CaseName("missing_demanded_right_source_refuses_instead_of_empty_success"),
        word: Word::Refuses,
        input: || Scenario::MissingRight,
        expect: Expect::Error(OperatingError::NotRead),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

#[cfg(test)]
mod source_partition_test;
