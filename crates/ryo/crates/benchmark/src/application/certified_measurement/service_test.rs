//! A pair is certified only when both contestants report the same findings.
use super::service::pair;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::evidence::fixtures::fake::EvidenceFake;
use crate::outbound::worker::fixtures::data::report;
use ryo_benchmark_contracts::domain::measurements::types::{Measurement, ResourceObservation};
use ryo_benchmark_contracts::domain::reports::errors::ReportsError as Error;

/// Whether the candidate lost one finding the reference reported.
type Input = bool;
type Value = usize;
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Refusal {
    Contract,
    Unexpected,
}
type Failure = Refusal;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("contestants reporting the same findings are paired"),
        word: Word::Succeeds,
        input: || false,
        expect: Expect::Value(2),
    },
    Case {
        name: CaseName("a candidate missing a finding is refused under the diagnostics contract"),
        word: Word::Refuses,
        input: || true,
        expect: Expect::Error(Refusal::Contract),
    },
];

fn measurement(report: ryo_benchmark_contracts::domain::reports::types::RunReport) -> Measurement {
    Measurement {
        wall: std::time::Duration::ZERO,
        resources: ResourceObservation::WallOnly,
        report,
    }
}

fn subject(lost_finding: Input) -> Result<Value, Failure> {
    let mut candidate = report();
    if lost_finding {
        let _lost = candidate.diagnostics.pop();
    }
    pair(&EvidenceFake, measurement(report()), measurement(candidate))
        .map(|paired| paired.reference().report.diagnostics.len())
        .map_err(|error| match error {
            Error::Invalid(message) if message.contains("diagnostic benchmark contract failed") => {
                Refusal::Contract
            }
            Error::Invalid(_) | Error::Io(_) | Error::Json(_) | Error::Toml(_) => {
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
