//! One catalog contract helper used by the inbound case table.
use super::super::interface::MetadataUseCase;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::query_operators::types::QueryOperator;

type Input = QueryOperator;
type Value = usize;
type Failure = ();

pub const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("select has its own public description"),
    word: Word::Succeeds,
    input: || QueryOperator::Select,
    expect: Expect::Value(6),
}];

/// How many distinct, nonempty public names the catalog's operators have: one per listed operator when each has its own.
///
/// # Panics
///
/// When a case fails.
pub fn check(subject: &impl MetadataUseCase) -> usize {
    run(CASES, Word::Succeeds, |operator| {
        Ok::<_, Failure>(subject.describe(operator).name.0.len())
    });
    subject
        .operators()
        .iter()
        .map(|&operator| subject.describe(operator).name.0)
        .filter(|name| !name.is_empty())
        .collect::<std::collections::BTreeSet<_>>()
        .len()
}
