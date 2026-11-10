//! An accounted run is admitted only against an audit that accounts for its inventory.
use super::{
    fixtures::tool, interface::AdmitMeasurementUseCase as _, types::MeasurementAdmission,
    use_case::AdmitMeasurementUseCaseImpl,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::measurements::types::MeasurementMode;
use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
use ryo_benchmark_contracts::domain::requests::types::BenchmarkCoverage;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    AccountedInventory,
    DiagnosticSubset,
    AccountedWithoutAuthoredRules,
}
type Input = Scenario;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Admitted {
    Benchmark(usize),
    Diagnostic,
}
type Value = Admitted;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    Invalid,
    Unexpected,
}
type Failure = Refusal;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("an accounted run is held to the declared inventory"),
        word: Word::Succeeds,
        input: || Scenario::AccountedInventory,
        expect: Expect::Value(Admitted::Benchmark(1)),
    },
    Case {
        name: CaseName("a diagnostic run is held to nothing"),
        word: Word::Succeeds,
        input: || Scenario::DiagnosticSubset,
        expect: Expect::Value(Admitted::Diagnostic),
    },
    Case {
        name: CaseName("an accounted run without an authored rule is refused"),
        word: Word::Refuses,
        input: || Scenario::AccountedWithoutAuthoredRules,
        expect: Expect::Error(Refusal::Invalid),
    },
];

fn subject(scenario: Input) -> Result<Value, Failure> {
    let (scope, authored) = match scenario {
        Scenario::AccountedInventory => (BenchmarkCoverage::Accounted, true),
        Scenario::DiagnosticSubset => (BenchmarkCoverage::Interim, true),
        Scenario::AccountedWithoutAuthoredRules => (BenchmarkCoverage::Accounted, false),
    };
    let tool = tool();
    let audit = crate::inbound::use_cases::audit::fixtures::audited(authored);
    AdmitMeasurementUseCaseImpl::new()
        .admit_measurement(&MeasurementAdmission {
            scope,
            audit: &audit,
            tool: &tool,
            declared_inventory: tool.defaults.clone(),
        })
        .map(|mode| match mode {
            MeasurementMode::Benchmark { inventory } => Admitted::Benchmark(inventory.len()),
            MeasurementMode::Diagnostic => Admitted::Diagnostic,
        })
        .map_err(|error| match error {
            ReportsError::Invalid(_) => Refusal::Invalid,
            ReportsError::Io(_) | ReportsError::Json(_) | ReportsError::Toml(_) => {
                Refusal::Unexpected
            }
        })
}

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}
