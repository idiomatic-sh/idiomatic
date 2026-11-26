//! The `format_parsing` service's cases: clearing leaves a fresh empty index.
use super::service::clear;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::format_syntax::types::{CachedFormat, FormatParseResult, FormatParseState, RetainedFormatIndex};

/// Nothing is given: the index starts empty.
type Input = ();
/// The state and the occurrence count after clearing, as text.
type Value = &'static str;
/// Nothing is refused.
type Failure = ();

/// The rows of this concept's cases.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("clearing_leaves_a_fresh_empty_index"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value("Fresh|0"),
}];

fn subject((): Input) -> Result<Value, Failure> {
    let mut index: RetainedFormatIndex<FormatParseResult, CachedFormat, FormatParseState> = RetainedFormatIndex::default();
    clear(&mut index);
    Ok(Box::leak(format!("{:?}|{}", index.state, index.occurrences.len()).into_boxed_str()))
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
