//! Actual public suite verification over independent source inputs and real child processes.
use super::fixtures::{Refusal, Scenario, exercise};
use super::{interface::VerifyCasesUseCase as _, use_case::VerifyCasesUseCaseImpl};
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
    Case {
        name: CaseName("removed query refuses before either contestant executes"),
        word: Word::Refuses,
        input: || Scenario::Unaccounted,
        expect: Expect::Error(Refusal::Unaccounted),
    },
];
fn verify(
    suite: &dyn SuitePort,
    reference: &dyn LinterPort,
    candidate: &dyn LinterPort,
    execution: &dyn ExecutionPort,
    rules: &Path,
    scratch: &Path,
) -> Result<usize, ReportsError> {
    let inventory = crate::outbound::inventory::factory::local();
    let context = crate::outbound::context::factory::local();
    let receipts = crate::outbound::receipts::fixtures::fake::ReceiptsFake::default();
    VerifyCasesUseCaseImpl::new(suite, &receipts, &context, reference, candidate, execution, &inventory)
        .verify_cases(
            rules,
            scratch,
            ryo_benchmark_contracts::domain::requests::types::CaseSelection::default(),
            ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage::Accounted,
        )
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

mod discrepancy;
mod metadata;
mod process_capture;
mod process_identity;
mod process_orchestration;
mod shard;
