//! Source-order selection over the current tree scan.
use super::service::select_tree_with_visits;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    operating::{errors::OperatingError, types::OperatorContext},
    positions::{
        service_test::{SMALL_SOURCE, rows_over, small_tree},
        types::Slots,
    },
    table_scratch::types::OperatorScratch,
    table_texts::types::SourceView,
};
use ryo_values::domain::plan_parts::types::{
    ColumnIndex, Comparison, LiteralText, Operand, QueryLiteral, Test,
};

type Input = fn() -> Result<Value, Failure>;
type Value = [u32; 2];
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("tree_selection_keeps_word_positions_in_source_order"),
    word: Word::Succeeds,
    input: || word_positions,
    expect: Expect::Value([1, 3]),
}];
fn word_positions() -> Result<Value, Failure> {
    let fixture = small_tree();
    let mut symbols = fixture.symbols;
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
    let comparison = Comparison {
        test: Test::Equal,
        left: Operand::Column(ColumnIndex(1)),
        right: Operand::Literal(QueryLiteral::Text(LiteralText("word".into()))),
        origin: None,
    };
    let mut visited = Vec::new();
    let mut selected = Slots::default();
    select_tree_with_visits(
        &rows_over(&fixture.elements),
        &[comparison],
        &mut context,
        &mut |at| visited.push(at.0),
        &mut selected,
    )?;
    assert!(visited.windows(2).all(|pair| pair[0] < pair[1]));
    let [first, second] = selected.0.as_slice() else {
        return Err(OperatingError::ShapeMismatch);
    };
    Ok([*first, *second])
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

fn subject(input: Input) -> Result<Value, Failure> {
    input()
}
