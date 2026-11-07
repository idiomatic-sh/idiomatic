//! Only samples issued from a complete inventory reach the evidence boundary, and never an empty set.
use super::constants;
use super::service::publish;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::discrepancy::fixtures::report;
use crate::domain::measurements::types::{MeasurementEligibility, sample::value::BenchmarkSample};
use crate::outbound::evidence::fixtures::fake::EvidenceFake;
use ryo_benchmark_contracts::domain::measurements::types::{
    Measurement, ResidentBytes, ResourceObservation, Resources, SampleName,
};
use std::time::Duration;

/// Which samples a row publishes.
#[derive(Clone, Copy)]
enum Samples {
    Complete,
    Empty,
    Diagnostic,
}

/// What a published set established.
#[derive(Debug, PartialEq, Eq)]
enum Published {
    Accepted,
}

/// What a refused publication states.
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    NoEvidence,
    DiagnosticOnly,
    Unexpected,
}

type Input = Samples;
type Value = Published;
type Failure = Refusal;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("complete inventory samples publish through the evidence boundary"),
        word: Word::Succeeds,
        input: || Samples::Complete,
        expect: Expect::Value(Published::Accepted),
    },
    Case {
        name: CaseName("an empty sample set claims no evidence and is refused"),
        word: Word::Refuses,
        input: || Samples::Empty,
        expect: Expect::Error(Refusal::NoEvidence),
    },
    Case {
        name: CaseName("a diagnostic sample cannot publish benchmark metrics"),
        word: Word::Refuses,
        input: || Samples::Diagnostic,
        expect: Expect::Error(Refusal::DiagnosticOnly),
    },
];

fn sample(eligibility: MeasurementEligibility) -> BenchmarkSample {
    BenchmarkSample {
        eligibility,
        name: SampleName("project/reference/cold".into()),
        measurement: Measurement {
            wall: Duration::from_millis(1),
            resources: ResourceObservation::Collected(Resources {
                cpu: Duration::from_millis(1),
                peak_rss_bytes: ResidentBytes(1),
            }),
            report: report(),
        },
    }
}

fn subject(samples: Input) -> Result<Value, Failure> {
    let issued = match samples {
        Samples::Complete => vec![sample(MeasurementEligibility::CompleteInventory)],
        Samples::Empty => Vec::new(),
        Samples::Diagnostic => vec![
            sample(MeasurementEligibility::CompleteInventory),
            sample(MeasurementEligibility::Diagnostic),
        ],
    };
    publish(&EvidenceFake, &issued)
        .map(|()| Published::Accepted)
        .map_err(|error| {
            let message = error.to_string();
            if message == constants::NO_MEASURED_EVIDENCE {
                Refusal::NoEvidence
            } else if message == constants::DIAGNOSTIC_OBSERVATIONS_CANNOT_PUBLISH_BENCH {
                Refusal::DiagnosticOnly
            } else {
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
