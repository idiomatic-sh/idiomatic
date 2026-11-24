//! An accounted run is admitted only against an audit that accounts for its inventory.
use super::{
    fixtures::tool, interface::AdmitMeasurementUseCase as _, types::MeasurementAdmission,
    use_case::AdmitMeasurementUseCaseImpl,
};
use crate::cases::{Case, CaseName, Expect, Word, run};
use benchmark_contracts::domain::measurements::types::MeasurementMode;
use benchmark_contracts::domain::reports::errors::ReportsError;
use benchmark_contracts::domain::requests::types::BenchmarkCoverage;

#[derive(Clone, Copy, Debug)]
enum Scenario {
    AccountedInventory,
    DiagnosticSubset,
    AccountedWithoutAuthoredRules,
    /// The tool defers a second default, which the audit finds unauthored.
    AccountedWithDeferral,
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
        name: CaseName(
            "an accounted run is held to the declared inventory but its deferred defaults",
        ),
        word: Word::Succeeds,
        input: || Scenario::AccountedWithDeferral,
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
        Scenario::AccountedInventory | Scenario::AccountedWithDeferral => {
            (BenchmarkCoverage::Accounted, true)
        }
        Scenario::DiagnosticSubset => (BenchmarkCoverage::Interim, true),
        Scenario::AccountedWithoutAuthoredRules => (BenchmarkCoverage::Accounted, false),
    };
    let mut tool = tool();
    let mut audit = crate::inbound::use_cases::audit::fixtures::audited(authored);
    if let Scenario::AccountedWithDeferral = scenario {
        let later =
            ryo::linter::RuleId::try_from("later").map_err(|_invalid| Refusal::Unexpected)?;
        tool.defaults.push(later.clone());
        let _previous = tool.deferred.insert(
            later.clone(),
            benchmark_contracts::domain::workloads::types::capability_name::value::CapabilityName(
                "a capability".into(),
            ),
        );
        audit.missing.push(later);
    }
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
