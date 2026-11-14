//! The `use_case` adapter of the `operators` port: the core's contract rows run through it.
use super::adapter::OperatorsUseCaseAdapter;
use crate::outbound::operators::fixtures::contract;
use ryo_operators::inbound::use_cases::operators::use_case::OperatorsUseCaseImpl;
use ryo_values::cases::{Case, CaseName, Expect, Word, run};
use ryo_values::domain::plan_parts::types::ColumnIndex;

type Input = ColumnIndex;
type Value = ColumnIndex;
type Failure = ();

/// What is specific to this adapter: it keeps nothing of its own, so a column index passes through it unchanged.
const CASES: &[Case<Input, Value, Failure>] = &[Case {
    name: CaseName("the_adapter_holds_the_use_case_it_was_built_over"),
    word: Word::Succeeds,
    input: || ColumnIndex(2),
    expect: Expect::Value(ColumnIndex(2)),
}];

fn subject(column: Input) -> Result<Value, Failure> {
    let matching = || std::sync::Arc::new(ryo_operators::outbound::matching::factory::use_case());
    contract::check(&OperatorsUseCaseAdapter::new(
        OperatorsUseCaseImpl::new(matching()),
        ryo_operators::inbound::use_cases::plan::use_case::PlanUseCaseImpl::new(),
        ryo_operators::inbound::use_cases::run::use_case::RunUseCaseImpl::new(matching()),
    ));
    Ok(column)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
