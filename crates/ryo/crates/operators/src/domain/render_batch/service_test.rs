//! The `render_batch` service's cases, on values alone.

use super::service::lower;
use super::types::RenderParts;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_language::domain::semantics::types::RenderContext;
use ryo_values::domain::plan_parts::types::ColumnIndex;

/// A scenario: whether what the part answers is what the case states.
type Input = fn() -> bool;
type Value = bool;
type Failure = ();

/// A render reads its node column and renders in its context.
fn renders_its_node_in_context() -> bool {
    lower(ColumnIndex(4), RenderContext::AttributeReceiver)
        == RenderParts {
            node: ColumnIndex(4),
            context: RenderContext::AttributeReceiver,
        }
}

/// The rows of this part's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("a_render_reads_its_node_in_its_context"),
    word: Word::Succeeds,
    input: || renders_its_node_in_context,
    expect: Expect::Value(true),
}];

fn subject(input: Input) -> Result<Value, Failure> {
    Ok(input())
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
