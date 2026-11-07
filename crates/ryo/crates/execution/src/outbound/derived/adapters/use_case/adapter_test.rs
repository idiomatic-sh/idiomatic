//! The `use_case` adapter of the `derived` port: the core's contract rows run through it.
use super::adapter::DerivedUseCaseAdapter;
use super::types::DerivedUseCases;
use crate::outbound::derived::fixtures::contract;
use ryo_derived::inbound::use_cases::build::use_case::BuildUseCaseImpl;
use ryo_derived::inbound::use_cases::prepare::use_case::PrepareUseCaseImpl;
use ryo_derived::inbound::use_cases::view::use_case::ViewUseCaseImpl;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;

type Input = ColumnIndex;
type Value = ColumnIndex;
type Failure = ();

/// What is specific to this adapter: it keeps nothing of its own, so a column index passes through it unchanged.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_adapter_holds_the_use_cases_it_was_built_over"),
    word: Word::Succeeds,
    input: || ColumnIndex(2),
    expect: Expect::Value(ColumnIndex(2)),
}];

fn subject(column: Input) -> Result<Value, Failure> {
    contract::check(&DerivedUseCaseAdapter::new(DerivedUseCases {
        prepare: std::sync::Arc::new(PrepareUseCaseImpl::new()),
        build: std::sync::Arc::new(BuildUseCaseImpl::new()),
        view: std::sync::Arc::new(ViewUseCaseImpl::new()),
    }));
    Ok(column)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
