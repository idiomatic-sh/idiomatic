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

/// Every listed operator has one nonempty, distinct public description.
///
/// # Panics
///
/// When a case fails, an operator's description is empty or repeats another's, or the catalog does not list 25 operators.
pub fn check(subject: &impl MetadataUseCase) -> usize {
    run(CASES, Word::Succeeds, |operator| {
        Ok::<_, Failure>(subject.describe(operator).name.0.len())
    });
    let operators = subject.operators();
    let mut names = std::collections::BTreeSet::new();
    for &operator in operators {
        let description = subject.describe(operator);
        assert!(!description.name.0.is_empty());
        assert!(names.insert(description.name.0));
    }
    assert_eq!(operators.len(), 25);
    names.len()
}
