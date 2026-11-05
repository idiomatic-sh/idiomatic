//! Ordered Join, Union and Antijoin rows through the native-position evaluator.

use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::native_positions::{
    service::evaluate,
    types::{PositionInput, PositionWork},
};
use ryo_operators::inbound::use_cases::operators::interface::{
    OperatorAnswer, OperatorCall, PortableOperatorFault,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::{
    plan_parts::types::{ColumnIndex, Input as PlanInput, JoinColumn, Side, StepId},
    plans::types::Operation,
};
use {
    crate::domain::lowering::types::ElementPart, crate::domain::lowering::types::LoweredColumn,
    crate::domain::lowering::types::LoweredShape, crate::domain::lowering::types::RowPosition,
    crate::domain::lowering::types::RowWidth, crate::domain::operating::errors::OperatingError,
    crate::domain::operating::types::OperatorContext,
    crate::domain::operator::contract::types::PrimitiveSemantics,
    crate::domain::operator::contract::types::WorkCounters,
    crate::domain::positions::service_test::SMALL_SOURCE,
    crate::domain::positions::service_test::rows_over,
    crate::domain::positions::service_test::small_tree, crate::domain::positions::types::Positions,
    crate::domain::positions::types::Slots, crate::domain::table_scratch::types::OperatorScratch,
    ryo_values::domain::files::types::SourceView,
};

struct Portable;
impl PrimitiveSemantics for Portable {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        crate::outbound::operators::factory::test_port().operators(call)
    }
}

#[derive(Clone, Copy)]
enum Scenario {
    Antijoin,
    MissingRight,
    Union,
    JoinSteps,
    JoinTree,
    UnsupportedRelation,
}
type Input = Scenario;
type Value = &'static str;
type Failure = OperatingError;

fn shape(field: ElementPart, width: usize) -> LoweredShape {
    LoweredShape {
        width: RowWidth(width),
        columns: vec![LoweredColumn {
            slot: RowPosition(0),
            field,
        }],
    }
}
fn positions(words: &[u32]) -> Positions {
    Positions {
        width: RowWidth(1),
        slots: Slots(words.to_vec()),
    }
}
fn join(right: PlanInput) -> Operation {
    Operation::Join {
        left: PlanInput::Step(StepId(0)),
        right,
        left_on: vec![ColumnIndex(0)],
        right_on: vec![ColumnIndex(0)],
        output: vec![JoinColumn {
            side: Side::Left,
            column: ColumnIndex(0),
        }],
        filter: vec![],
        view: vec![],
        reach: None,
    }
}
fn subject(scenario: Input) -> Result<Value, Failure> {
    let fixture = small_tree();
    let mut symbols = fixture.symbols;
    let tree = rows_over(&fixture.elements);
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
    let kind = shape(ElementPart::Kind, 1);
    let node = shape(ElementPart::Node, 1);
    let left_all = positions(&[1, 2, 3]);
    let left_words = positions(&[1, 3]);
    let right_one = positions(&[1]);
    let left_tree = positions(&[2]);
    let right_other = positions(&[3]);
    let left_union = positions(&[1, 2]);
    let (operation, left, right, output) = match scenario {
        Scenario::Antijoin | Scenario::MissingRight => (
            Operation::Antijoin {
                left: PlanInput::Step(StepId(0)),
                right: PlanInput::Step(StepId(1)),
                left_on: vec![ColumnIndex(0)],
                right_on: vec![ColumnIndex(0)],
            },
            PositionInput {
                shape: &kind,
                rows: &left_all,
            },
            matches!(scenario, Scenario::Antijoin).then_some(PositionInput {
                shape: &kind,
                rows: &right_one,
            }),
            kind.clone(),
        ),
        Scenario::Union => (
            Operation::Union {
                left: PlanInput::Step(StepId(0)),
                right: PlanInput::Step(StepId(1)),
            },
            PositionInput {
                shape: &kind,
                rows: &left_union,
            },
            Some(PositionInput {
                shape: &kind,
                rows: &right_other,
            }),
            shape(ElementPart::Kind, 1),
        ),
        Scenario::JoinSteps => (
            join(PlanInput::Step(StepId(1))),
            PositionInput {
                shape: &kind,
                rows: &left_words,
            },
            Some(PositionInput {
                shape: &kind,
                rows: &right_one,
            }),
            shape(ElementPart::Kind, 2),
        ),
        Scenario::JoinTree | Scenario::UnsupportedRelation => (
            join(PlanInput::Relation(
                if matches!(scenario, Scenario::JoinTree) {
                    Relation::SyntaxTree
                } else {
                    Relation::Function
                },
            )),
            PositionInput {
                shape: &node,
                rows: &left_tree,
            },
            None,
            shape(ElementPart::Node, 2),
        ),
    };
    let mut inputs = vec![left];
    inputs.extend(right);
    let mut derived = crate::application::file_dag::testing::raw_scope()?;
    let result = evaluate(
        &Portable,
        &mut derived,
        &operation,
        &tree,
        &inputs,
        &output,
        &mut context,
        &mut PositionWork::default(),
        &WorkCounters::default(),
    )?;
    Ok(Box::leak(
        result
            .slots
            .0
            .iter()
            .map(u32::to_string)
            .collect::<Vec<_>>()
            .join(" ")
            .into_boxed_str(),
    ))
}
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("native_antijoin_preserves_missing_key_rows"),
        word: Word::Succeeds,
        input: || Scenario::Antijoin,
        expect: Expect::Value("2"),
    },
    Case {
        name: CaseName("native_union_preserves_first_distinct_keys"),
        word: Word::Succeeds,
        input: || Scenario::Union,
        expect: Expect::Value("1 2"),
    },
    Case {
        name: CaseName("native_join_pairs_each_matching_step_row"),
        word: Word::Succeeds,
        input: || Scenario::JoinSteps,
        expect: Expect::Value("1 1 3 1"),
    },
    Case {
        name: CaseName("native_join_follows_tree_node"),
        word: Word::Succeeds,
        input: || Scenario::JoinTree,
        expect: Expect::Value("2 2"),
    },
    Case {
        name: CaseName("native_antijoin_refuses_missing_required_right_operand"),
        word: Word::Refuses,
        input: || Scenario::MissingRight,
        expect: Expect::Error(OperatingError::MissingInput { position: 2 }),
    },
    Case {
        name: CaseName("native_join_declines_non_tree_relation_without_running_it"),
        word: Word::Refuses,
        input: || Scenario::UnsupportedRelation,
        expect: Expect::Error(OperatingError::NotAStep),
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
