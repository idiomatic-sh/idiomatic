//! Selection and projection results through the current borrowed native evaluator.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    lowering::types::{ElementPart, LoweredColumn, LoweredShape, RowPosition, RowWidth},
    native_positions::{
        service::evaluate,
        types::{PositionInput, PositionWork},
    },
    operating::{errors::OperatingError, types::OperatorContext},
    operator::contract::types::{PrimitiveSemantics, WorkCounters},
    positions::{
        service_test::{SMALL_SOURCE, rows_over, small_tree},
        types::{Positions, Slots},
    },
    table_scratch::types::OperatorScratch,
    table_texts::types::SourceView,
};
use ryo_operators::domain::contract::types::{OperatorAnswer, OperatorCall, PortableOperatorFault};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::{
    plan_parts::types::{
        ColumnIndex, Comparison, Input as PlanInput, LiteralText, Operand, QueryLiteral,
        RepeatedRows, StepId, Test,
    },
    plans::types::Operation,
};

struct Portable;
impl PrimitiveSemantics for Portable {
    fn operators(&self, call: OperatorCall<'_>) -> Result<OperatorAnswer, PortableOperatorFault> {
        crate::outbound::operators::factory::test_port().operators(call)
    }
}
#[derive(Clone, Copy)]
enum Scenario {
    TreeWords,
    RowsOfWords,
    Distinct,
    Kept,
}
type Value = &'static str;
type Failure = OperatingError;
type Input = Scenario;

fn kind_is_word(column: u32) -> Comparison {
    Comparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(column)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText("word".into()))),
        origin: None,
    }
}
fn subject(scenario: Input) -> Result<Value, Failure> {
    let fixture = small_tree();
    let mut symbols = fixture.symbols;
    let tree = rows_over(&fixture.elements);
    let shape = LoweredShape {
        width: RowWidth(1),
        columns: vec![LoweredColumn {
            slot: RowPosition(0),
            field: ElementPart::Kind,
        }],
    };
    let rows = Positions {
        width: RowWidth(1),
        slots: Slots(vec![1, 2, 3]),
    };
    let input = PositionInput {
        shape: &shape,
        rows: &rows,
    };
    let operation = match scenario {
        Scenario::TreeWords => Operation::Select {
            from: PlanInput::Relation(Relation::SyntaxTree),
            comparisons: vec![kind_is_word(1)],
            kept: vec![ColumnIndex(1)],
        },
        Scenario::RowsOfWords => Operation::Select {
            from: PlanInput::Step(StepId(0)),
            comparisons: vec![kind_is_word(0)],
            kept: vec![ColumnIndex(0)],
        },
        Scenario::Distinct | Scenario::Kept => Operation::Project {
            from: PlanInput::Step(StepId(0)),
            columns: vec![ColumnIndex(0)],
            distinct: if matches!(scenario, Scenario::Distinct) {
                RepeatedRows::Dropped
            } else {
                RepeatedRows::Kept
            },
        },
    };
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
    let mut derived = crate::application::file_dag::testing::raw_scope()?;
    let inputs = if matches!(scenario, Scenario::TreeWords) {
        &[][..]
    } else {
        std::slice::from_ref(&input)
    };
    let selected = evaluate(
        &Portable,
        &mut derived,
        &operation,
        &tree,
        inputs,
        &shape,
        &mut context,
        &mut PositionWork::default(),
        &WorkCounters::default(),
    )?;
    Ok(Box::leak(
        selected
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
        name: CaseName("the_elements_of_the_tree_of_a_kind_are_selected"),
        word: Word::Succeeds,
        input: || Scenario::TreeWords,
        expect: Expect::Value("1 3"),
    },
    Case {
        name: CaseName("the_rows_of_a_step_whose_element_is_of_a_kind_are_selected"),
        word: Word::Succeeds,
        input: || Scenario::RowsOfWords,
        expect: Expect::Value("1 3"),
    },
    Case {
        name: CaseName("a_projection_that_drops_repeats_keeps_the_first_row_of_each_kind"),
        word: Word::Succeeds,
        input: || Scenario::Distinct,
        expect: Expect::Value("1 2"),
    },
    Case {
        name: CaseName("a_projection_that_keeps_repeats_keeps_every_row"),
        word: Word::Succeeds,
        input: || Scenario::Kept,
        expect: Expect::Value("1 2 3"),
    },
];
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
