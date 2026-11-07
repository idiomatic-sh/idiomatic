//! Actual public suite verification over independent source inputs and real child processes.
use super::fixtures::{Refusal, Scenario, exercise};
use super::{
    interface::VerifyReferenceCasesUseCase as _, use_case::VerifyReferenceCasesUseCaseImpl,
};
use crate::outbound::{
    execution::interface::ExecutionPort, linter::interface::LinterPort, suite::interface::SuitePort,
};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use std::path::Path;
use crate::cases::{Case, CaseName, Expect, Word, run};
type Input = Scenario;
type Value = usize;
type Failure = Refusal;
const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("two authored cases execute through the actual public operation"),
        word: Word::Succeeds,
        input: || Scenario::Complete,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("missing sources refuse before any process executes"),
        word: Word::Refuses,
        input: || Scenario::Missing,
        expect: Expect::Error(Refusal::MissingSource),
    },
];
fn verify(
    suite: &dyn SuitePort,
    reference: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    rules: &Path,
    scratch: &Path,
) -> Result<usize, ReportsError> {
    let inventory = crate::outbound::inventory::factory::local();
    let context = crate::outbound::context::factory::local();
    let receipts = crate::outbound::receipts::fixtures::fake::ReceiptsFake::default();
    VerifyReferenceCasesUseCaseImpl::new(suite, &receipts, &context, reference, execution, &inventory)
        .verify_reference_cases(rules, scratch)
        .map(|count| count.0)
}
fn subject(input: Input) -> Result<Value, Failure> {
    exercise(input, verify)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

mod native;
pub(crate) mod process_capture;
pub(crate) mod process_identity;
mod reference;
