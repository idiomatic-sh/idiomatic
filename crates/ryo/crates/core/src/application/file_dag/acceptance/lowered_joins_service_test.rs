//! The `joins` service's cases, over the small tree.

use crate::application::file_dag::acceptance::fragment_oracle::service::set_rows::{
    antijoin, union,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::lowering::types::ElementPart;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operating::types::OperatorContext;
use crate::domain::positions::service_test::{
    SMALL_SOURCE, SmallTree, lowering_of, room_of, rows_over, small_tree,
};
use crate::domain::positions::types::{KeyColumns, Sides, Slots};
use crate::domain::table_scratch::types::OperatorScratch;
use crate::domain::table_texts::types::SourceView;
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::ColumnIndex;
use ryo_values::domain::plan_parts::types::Input as PlanInput;
use ryo_values::domain::plan_parts::types::JoinColumn;
use ryo_values::domain::plan_parts::types::Side;
use ryo_values::domain::plan_parts::types::StepId;
use ryo_values::domain::plans::types::Operation;

/// What a row of the cases does.
struct JoinPortable;
impl crate::domain::operator::contract::types::PrimitiveSemantics for JoinPortable {
    fn operators(
        &self,
        call: ryo_operators::domain::contract::types::OperatorCall<'_>,
    ) -> Result<
        ryo_operators::domain::contract::types::OperatorAnswer,
        ryo_operators::domain::contract::types::PortableOperatorFault,
    > {
        crate::outbound::operators::factory::test_port().operators(call)
    }
}

#[derive(Clone, Copy)]
enum Scenario {
    /// The words and the other, minus the rows of the kind the right step has.
    Antijoin,
    /// An antijoin whose right step has made no rows: it has not run.
    AntijoinOfAStepNotRun,
    /// The rows of both steps, once each by their kind.
    Union,
    /// Each row of a step with the rows of another that are of the same kind.
    JoinSteps,
    /// Each row of a step with the element it names in the tree.
    JoinTree,
    /// A join whose right side is a relation that is no step and not the tree.
    JoinAnotherRelation,
}

/// What a row of the cases does.
type Input = Scenario;
/// The positions that remain, shown.
type Value = &'static str;
type Failure = OperatingError;

fn join_on(right: PlanInput) -> Operation {
    Operation::Join {
        left: PlanInput::Step(StepId(0)),
        right,
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        output: vec![JoinColumn {
            side: Side::Left,
            column: ColumnIndex(0),
        }],
        filter: Vec::new(),
        view: Vec::new(),

        reach: None,
    }
}

fn subject(scenario: Input) -> Result<Value, Failure> {
    let SmallTree {
        elements,
        mut symbols,
    } = small_tree();
    let rows = rows_over(&elements);
    let mut out = Slots::default();
    let columns = [ColumnIndex(0)];
    let semantics = JoinPortable;
    let mut derived = crate::application::file_dag::testing::raw_scope()?;
    match scenario {
        Scenario::Antijoin => antijoin(
            &semantics,
            &mut derived,
            &rows,
            &lowering_of(&[ElementPart::Kind, ElementPart::Kind]),
            Sides {
                left: StepId(0),
                right: StepId(1),
            },
            KeyColumns {
                left: &columns,
                right: &columns,
            },
            &mut room_of(&[&[1, 2, 3], &[1]]),
            &mut out,
        )?,
        Scenario::AntijoinOfAStepNotRun => antijoin(
            &semantics,
            &mut derived,
            &rows,
            &lowering_of(&[ElementPart::Kind, ElementPart::Kind]),
            Sides {
                left: StepId(0),
                right: StepId(1),
            },
            KeyColumns {
                left: &columns,
                right: &columns,
            },
            &mut room_of(&[&[1, 2, 3]]),
            &mut out,
        )?,
        Scenario::Union => union(
            &semantics,
            &mut derived,
            &rows,
            &lowering_of(&[ElementPart::Kind, ElementPart::Kind, ElementPart::Kind]),
            StepId(2),
            Sides {
                left: StepId(0),
                right: StepId(1),
            },
            &mut room_of(&[&[1, 2], &[3], &[]]),
            &mut out,
        )?,
        Scenario::JoinSteps | Scenario::JoinTree | Scenario::JoinAnotherRelation => {
            let mut scratch = OperatorScratch::default();
            let mut memo = Vec::new();
            let mut context = OperatorContext {
                semantic: None,
                prepared_kinds: None,
                prepared_matchers: None,
                bindings: &[],
                symbols: (&mut symbols).into(),
                source: SourceView(SMALL_SOURCE),
                paths: &[],
                arenas: &[],
                memo: &mut memo,
                scratch: &mut scratch,
            };
            let (fields, right, mut room) = if matches!(scenario, Scenario::JoinSteps) {
                (
                    vec![ElementPart::Kind, ElementPart::Kind],
                    PlanInput::Step(StepId(1)),
                    room_of(&[&[1, 3], &[1]]),
                )
            } else if matches!(scenario, Scenario::JoinAnotherRelation) {
                (
                    vec![ElementPart::Node],
                    PlanInput::Relation(Relation::Function),
                    room_of(&[&[2]]),
                )
            } else {
                (
                    vec![ElementPart::Node],
                    PlanInput::Relation(Relation::SyntaxTree),
                    room_of(&[&[2]]),
                )
            };
            use crate::domain::lowering::types::{LoweredShape, RowWidth};
            use crate::domain::positions::types::{Positions, Readiness};
            use ryo_values::domain::plans::types::{Emit, Plan, PlanStep, StepScope};
            let mut lowering = lowering_of(&fields);
            let root = StepId(u32::try_from(room.made.len()).map_err(|_| OperatingError::NotRun)?);
            let shape = lowering.shapes[0].as_ref().expect("left shape").clone();
            lowering.shapes.push(Some(LoweredShape {
                width: RowWidth(2),
                columns: shape.columns,
            }));
            lowering
                .counting
                .push(crate::domain::lowering::types::Counting::DoesNotCount);
            let mut steps = (0..room.made.len())
                .map(|_| PlanStep {
                    operation: Operation::Select {
                        from: PlanInput::Relation(Relation::SyntaxTree),
                        comparisons: vec![],
                        kept: vec![],
                    },
                    scope: StepScope::PerFile,
                })
                .collect::<Vec<_>>();
            steps.push(PlanStep {
                operation: join_on(right),
                scope: StepScope::PerFile,
            });
            let plan = Plan {
                observation_inputs: Vec::new(),
                observations: Vec::new(),
                parameters: vec![],
                steps,
                emit: Emit {
                    step: root,
                    subject: ColumnIndex(0),
                    key: vec![],
                    headings: vec![],
                },
                frontier: vec![],
                comparison_origins: vec![],
            };
            room.made.push(Positions::default());
            room.ready.push(Readiness::NotMade);
            let mut derived = crate::application::file_dag::testing::raw_scope()?;
            let mut run =
                crate::application::file_dag::acceptance::fragment_oracle::types::FragmentRun {
                    derived: &mut derived,
                    plan: &plan,
                    lowering: &lowering,
                    tree: rows,
                    room: &mut room,
                    classes: &[],
                };
            let semantics = JoinPortable;
            crate::application::file_dag::acceptance::fragment_oracle::service::make(
                &semantics,
                &mut run,
                root,
                &mut context,
            )?;
            out = run
                .room
                .made
                .last()
                .expect("joined positions")
                .slots
                .clone();
        }
    }
    // The answer is leaked: a case compares it with text that lives for the whole run.
    Ok(Box::leak(
        out.0
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(" ")
            .into_boxed_str(),
    ))
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an_antijoin_keeps_the_rows_whose_key_the_right_step_lacks"),
        word: Word::Succeeds,
        input: || Scenario::Antijoin,
        expect: Expect::Value("2"),
    },
    Case {
        name: CaseName("an_antijoin_of_a_step_that_has_not_run_is_refused_not_empty"),
        word: Word::Refuses,
        input: || Scenario::AntijoinOfAStepNotRun,
        expect: Expect::Error(OperatingError::NotRun),
    },
    Case {
        name: CaseName("a_join_with_a_relation_that_is_no_step_is_refused_as_not_a_step"),
        word: Word::Refuses,
        input: || Scenario::JoinAnotherRelation,
        expect: Expect::Error(OperatingError::NotAStep),
    },
    Case {
        name: CaseName("a_union_keeps_each_row_once_by_its_key"),
        word: Word::Succeeds,
        input: || Scenario::Union,
        expect: Expect::Value("1 2"),
    },
    Case {
        name: CaseName("a_join_of_two_steps_pairs_the_rows_that_are_of_one_kind"),
        word: Word::Succeeds,
        input: || Scenario::JoinSteps,
        expect: Expect::Value("1 1 3 1"),
    },
    Case {
        name: CaseName("a_join_with_the_tree_follows_the_element_a_row_names"),
        word: Word::Succeeds,
        input: || Scenario::JoinTree,
        expect: Expect::Value("2 2"),
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
