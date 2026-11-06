//! Select, count and refusal cases through the prepared file program.

use crate::application::file_dag::execution::CommonOperatorEngine;
use crate::application::file_dag::program::facade_runtime::file_execution::open_file;
use crate::application::file_dag::program::facade_runtime::types::ProgramServices;
use crate::application::file_dag::program::protocol::types::CaptureIntent;
use crate::application::file_dag::program::protocol::types::DiagnosticDelivery;
use crate::application::file_dag::program::protocol::types::EmitDemand;
use crate::application::file_dag::program::protocol::types::FileRuleIntent;
use crate::application::file_dag::program::protocol::types::ObservationDemand;
use crate::application::file_dag::program::protocol::types::RootOutcome;
use crate::application::file_dag::program::protocol::types::RuleDemand;
use crate::application::file_dag::scope::ScopePartials;
use crate::domain::{
    file_program::types::RuleSchedule,
    operating::errors::OperatingError,
    positions::service_test::{SMALL_SOURCE, SmallTree, rows_over, small_tree},
    prepared_request::types::{Compiled, ObservationIdentity},
    table_model::types::Rows,
};
use ryo_language::domain::files::types::FilePath;
use ryo_relations::domain::relations::{
    service::{kinds, node_id},
    types::{CellKind, FileId, Flag, Integer, Offset, Position, Relation, Span, Value as Cell},
};
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::{
    files::types::{CorpusEntry, CorpusInput, ExecutablePermission, FileSize, Nanoseconds},
    plan_parts::types::{
        ColumnIndex, PlanComparison, Input as PlanInput, JoinColumn, LiteralText, Operand,
        QueryLiteral, RepeatedRows, Side, StepId, StepScope, Test,
    },
    plans::types::{Emit, Operation, Plan, PlanStep},
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
    RowsOfWords,
    DistinctKinds,
    KeptKinds,
    NativeAntijoin,
    NativeUnion,
    NativeJoinSteps,
    NativeJoinTree,
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
            } else if matches!(
                scenario,
                Scenario::RowsOfWords | Scenario::DistinctKinds | Scenario::KeptKinds
            ) {
                vec![PlanComparison {
                    test: Test::NotEqual,
                    left: Operand::Column(ColumnIndex(1)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("file".into()))),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }]
            } else {
                vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(1)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("word".into()))),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }]
            },
            kept: if matches!(scenario, Scenario::DistinctKinds | Scenario::KeptKinds) {
                vec![ColumnIndex(1)]
            } else {
                vec![ColumnIndex(0), ColumnIndex(1)]
            },
        },
        scope: StepScope::PerFile,
    }];
    if matches!(scenario, Scenario::RowsOfWords) {
        steps.push(PlanStep {
            operation: Operation::Select {
                from: PlanInput::Step(StepId(0)),
                comparisons: vec![PlanComparison {
                    test: Test::Equal,
                    left: Operand::Column(ColumnIndex(1)),
                    right: Operand::Literal(QueryLiteral::Text(LiteralText("word".into()))),
                    origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
                }],
                kept: vec![ColumnIndex(0)],
            },
            scope: StepScope::PerFile,
        });
    }
    if matches!(
        scenario,
        Scenario::RowsOfWords | Scenario::DistinctKinds | Scenario::KeptKinds
    ) {
        steps.push(PlanStep {
            operation: Operation::Project {
                from: PlanInput::Step(StepId((steps.len() - 1) as u32)),
                columns: vec![ColumnIndex(0)],
                distinct: if matches!(scenario, Scenario::KeptKinds) {
                    RepeatedRows::Kept
                } else {
                    RepeatedRows::Dropped
                },
            },
            scope: StepScope::PerFile,
        });
    }
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
    if matches!(
        scenario,
        Scenario::NativeAntijoin
            | Scenario::NativeUnion
            | Scenario::NativeJoinSteps
            | Scenario::NativeJoinTree
    ) {
        let select = |comparisons, kept| PlanStep {
            operation: Operation::Select {
                from: PlanInput::Relation(Relation::SyntaxTree),
                comparisons,
                kept,
            },
            scope: StepScope::PerFile,
        };
        let text_is = |column, test, text: &str| PlanComparison {
            test,
            left: Operand::Column(ColumnIndex(column)),
            right: Operand::Literal(QueryLiteral::Text(LiteralText(text.into()))),
            origin: ryo_values::domain::plan_parts::types::ComparisonOrigin::Unrecorded,
        };
        steps = match scenario {
            Scenario::NativeAntijoin | Scenario::NativeUnion => vec![
                select(
                    vec![text_is(1, Test::NotEqual, "file")],
                    vec![ColumnIndex(1)],
                ),
                select(vec![text_is(1, Test::Equal, "word")], vec![ColumnIndex(1)]),
                PlanStep {
                    operation: if matches!(scenario, Scenario::NativeAntijoin) {
                        Operation::Antijoin {
                            left: PlanInput::Step(StepId(0)),
                            right: PlanInput::Step(StepId(1)),
                            left_on: vec![ColumnIndex(0)],
                            right_on: vec![ColumnIndex(0)],
                        }
                    } else {
                        Operation::Union {
                            left: PlanInput::Step(StepId(0)),
                            right: PlanInput::Step(StepId(1)),
                        }
                    },
                    scope: StepScope::PerFile,
                },
            ],
            Scenario::NativeJoinSteps => vec![
                select(
                    vec![text_is(1, Test::Equal, "word")],
                    vec![ColumnIndex(0), ColumnIndex(1)],
                ),
                select(
                    vec![text_is(2, Test::Equal, "a")],
                    vec![ColumnIndex(0), ColumnIndex(1)],
                ),
                PlanStep {
                    operation: Operation::Join {
                        left: PlanInput::Step(StepId(0)),
                        right: PlanInput::Step(StepId(1)),
                        left_on: vec![ColumnIndex(1)],
                        right_on: vec![ColumnIndex(1)],
                        output: vec![
                            JoinColumn {
                                side: Side::Left,
                                column: ColumnIndex(0),
                            },
                            JoinColumn {
                                side: Side::Right,
                                column: ColumnIndex(0),
                            },
                        ],
                        filter: vec![],
                        view: vec![],
                        reach: None,
                    },
                    scope: StepScope::PerFile,
                },
            ],
            Scenario::NativeJoinTree => vec![
                select(vec![text_is(1, Test::Equal, "other")], vec![ColumnIndex(0)]),
                PlanStep {
                    operation: Operation::Join {
                        left: PlanInput::Step(StepId(0)),
                        right: PlanInput::Relation(Relation::SyntaxTree),
                        left_on: vec![ColumnIndex(0)],
                        right_on: vec![ColumnIndex(0)],
                        output: vec![
                            JoinColumn {
                                side: Side::Left,
                                column: ColumnIndex(0),
                            },
                            JoinColumn {
                                side: Side::Right,
                                column: ColumnIndex(0),
                            },
                        ],
                        filter: vec![],
                        view: vec![],
                        reach: None,
                    },
                    scope: StepScope::PerFile,
                },
            ],
            _ => unreachable!(),
        };
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
            modules: Default::default(),
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
    let prepared = crate::application::file_dag::program::preparation::service::prepare(
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
    let target = StepId((plan(scenario).steps.len() - 1) as u32);
    let target_region = {
        let program = prepared.program();
        let logical = program.rules()[0].original_steps[target.0 as usize]
            .as_ref()
            .unwrap()
            .logical;
        program.logical_regions()[logical.0]
    };
    let expects_native = matches!(
        scenario,
        Scenario::NativeAntijoin
            | Scenario::NativeUnion
            | Scenario::NativeJoinSteps
            | Scenario::NativeJoinTree
    );
    if expects_native {
        assert!(
            crate::domain::executable_program::service::native_positions(
                &prepared.program().regions()[target_region.0].route
            )
            .is_some(),
            "this case requires the compiled native-position strategy to be installed"
        );
    }
    if matches!(scenario, Scenario::OrdinaryFunctionJoin) {
        assert!(
            crate::domain::executable_program::service::native_positions(
                &prepared.program().regions()[target_region.0].route
            )
            .is_none(),
            "the non-tree RHS must decline native-position installation"
        );
    }
    let engine = Arc::new(CommonOperatorEngine::new(std::sync::Arc::new(
        ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl::new(),
    )));
    let services = Arc::new(ProgramServices::new(
        engine,
        derived,
        &[CorpusInput::Available(CorpusEntry {
            executable: ExecutablePermission::Unknown,
            path: FilePath("native-parity.py".into()),
            size: FileSize(SMALL_SOURCE.len() as u64),
            modified: Nanoseconds(0),
        })],
    ));
    let mut file = open_file(
        &prepared,
        FileId(0),
        ScopePartials::File {
            evaluation: &[],
            publication: &[],
        },
        &[FileRuleIntent::Run {
            diagnostic: DiagnosticDelivery::Collected,
            observations: ObservationDemand::NotRequired,
            capture: CaptureIntent::Disabled,
        }],
        services,
        ryo_values::domain::files::types::SourceView(SMALL_SOURCE),
        crate::application::file_dag::program::facade_runtime::types::ReuseEligibility::Eligible,
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
    let routes = file.route_observation(crate::domain::coverage::types::RuleIndex(0), target)?;
    if expects_native {
        assert!(
            routes.native_positions > 0,
            "published rows must come from actual native-position execution; ordinary parity alone is insufficient"
        );
        assert_eq!(
            routes.registered_calls, 0,
            "the target did not dispatch through ordinary registration"
        );
    }
    if matches!(scenario, Scenario::OrdinaryFunctionJoin) {
        assert_eq!(routes.native_positions, 0);
        assert!(
            routes.registered_calls > 0,
            "a declined native shape must execute the real registered operator"
        );
    }
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
            Cell::Symbol(symbol)
                if matches!(
                    scenario,
                    Scenario::DistinctKinds
                        | Scenario::KeptKinds
                        | Scenario::NativeAntijoin
                        | Scenario::NativeUnion
                ) =>
            {
                ryo_language::domain::symbols::service::text_of(
                    &symbols,
                    ryo_language::domain::symbols::types::ByteSlice(SMALL_SOURCE),
                    *symbol,
                )
                .map_or_else(|| "?".to_owned(), |text| text.0.to_owned())
            }
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
        name: CaseName("the_elements_of_the_tree_of_a_kind_are_selected"),
        word: Word::Succeeds,
        input: || Scenario::Words,
        expect: Expect::Value((2, "1 _ 3 _")),
    },
    Case {
        name: CaseName("the_rows_of_a_step_whose_element_is_of_a_kind_are_selected"),
        word: Word::Succeeds,
        input: || Scenario::RowsOfWords,
        expect: Expect::Value((2, "1 3")),
    },
    Case {
        name: CaseName("a_projection_that_drops_repeats_keeps_the_first_row_of_each_kind"),
        word: Word::Succeeds,
        input: || Scenario::DistinctKinds,
        expect: Expect::Value((2, "word other")),
    },
    Case {
        name: CaseName("a_projection_that_keeps_repeats_keeps_every_row"),
        word: Word::Succeeds,
        input: || Scenario::KeptKinds,
        expect: Expect::Value((3, "word other word")),
    },
    Case {
        name: CaseName("a_count_of_lowered_rows_is_passed_on_with_the_keys"),
        word: Word::Succeeds,
        input: || Scenario::Counted,
        expect: Expect::Value((1, "_ 2")),
    },
    Case {
        name: CaseName("native_antijoin_preserves_missing_key_rows"),
        word: Word::Succeeds,
        input: || Scenario::NativeAntijoin,
        expect: Expect::Value((1, "other")),
    },
    Case {
        name: CaseName("native_union_preserves_first_distinct_keys"),
        word: Word::Succeeds,
        input: || Scenario::NativeUnion,
        expect: Expect::Value((2, "word other")),
    },
    Case {
        name: CaseName("native_join_pairs_each_matching_step_row"),
        word: Word::Succeeds,
        input: || Scenario::NativeJoinSteps,
        expect: Expect::Value((2, "1 1 3 1")),
    },
    Case {
        name: CaseName("native_join_follows_tree_node"),
        word: Word::Succeeds,
        input: || Scenario::NativeJoinTree,
        expect: Expect::Value((1, "2 2")),
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
mod source_partition;

use ryo_values::domain::rule_requests::types::{RuleParameters, RuleSpec};
