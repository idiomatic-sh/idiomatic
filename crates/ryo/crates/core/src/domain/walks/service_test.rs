//! Marked tree-domain admission in the current native walk filter.
use super::{
    service::allows,
    types::{Admit, Marked},
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    operating::{errors::OperatingError, types::OperatorContext},
    positions::service_test::{SMALL_SOURCE, rows_over, small_tree},
    table_model::types::Verdict,
    table_scratch::types::OperatorScratch,
    table_texts::types::SourceView,
};
use ryo_relations::domain::relations::types::Position;

type Input = fn() -> Result<Value, Failure>;
type Value = [Verdict; 2];
type Failure = OperatingError;
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("marked_domain_admits_only_declared_tree_places"),
    word: Word::Succeeds,
    input: || marked_domain,
    expect: Expect::Value([Verdict::Holds, Verdict::Fails]),
}];
fn marked_domain() -> Result<Value, Failure> {
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
    let mut admission = Admit::Marks(Marked(vec![false, true, false, false]));
    Ok([
        allows(&mut admission, &tree, Position(1), &mut context)?,
        allows(&mut admission, &tree, Position(2), &mut context)?,
    ])
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}

fn subject(input: Input) -> Result<Value, Failure> {
    input()
}
