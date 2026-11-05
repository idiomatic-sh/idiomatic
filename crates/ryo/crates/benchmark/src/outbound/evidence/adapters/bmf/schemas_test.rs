//! Independent BMF expectations cover aggregation and absent observations.
use super as schemas;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::measurements::types::Measurement;
use ryo_benchmark_contracts::domain::measurements::types::Resources;
use ryo_benchmark_contracts::domain::measurements::types::{PublicationSample, SampleName};
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use std::time::Duration;

#[derive(Clone, Copy)]
enum Scenario {
    Resources,
    Absent,
    InexactResident,
}
type Input = Scenario;
type Value = serde_json::Value;
type Failure = Refusal;
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    Absent,
    Inexact,
    Unexpected,
}

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("resident-byte evidence never silently rounds an integer"),
        word: Word::Refuses,
        input: || Scenario::InexactResident,
        expect: Expect::Error(Failure::Inexact),
    },
    Case {
        name: CaseName("median and bounds preserve independent metric scales"),
        word: Word::Succeeds,
        input: || Scenario::Resources,
        expect: Expect::Value(serde_json::Value::Null),
    },
    Case {
        name: CaseName("wall only sample cannot masquerade as full evidence"),
        word: Word::Refuses,
        input: || Scenario::Absent,
        expect: Expect::Error(Failure::Absent),
    },
];
fn subject(scenario: Input) -> Result<serde_json::Value, Failure> {
    let samples: Vec<_> = [2, 8, 4, 6]
        .into_iter()
        .map(|millis| SampleInput {
            complete_inventory:true,
            name: ryo_benchmark_contracts::domain::measurements::types::SampleName(
                "project/ryo/empty-tool-cache".into(),
            ),
            measurement: Measurement {
                wall: Duration::from_millis(millis),
                resources: if matches!(scenario, Scenario::Absent) {
                    ryo_benchmark_contracts::domain::measurements::types::ResourceObservation::WallOnly
                } else {
                    ryo_benchmark_contracts::domain::measurements::types::ResourceObservation::Collected(Resources {
                        cpu: Duration::from_millis(millis * 2),
                        peak_rss_bytes: ryo_benchmark_contracts::domain::measurements::types::ResidentBytes(
                            if matches!(scenario, Scenario::InexactResident) {
                                super::super::constants::MAX_EXACT_RESIDENT + 1
                            } else {
                                millis * 1024
                            },
                        ),
                    })
                },
                report: RunReport {
                    execution_metadata:
                        ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Unknown,
                    project_context: None,
                    skipped: Vec::new(),
                    coverage: None,
                    diagnostics: Vec::new(),
                    files: vec![ryo::linter::RelativePath("input.py".into())],
                },
            },
        })
        .collect();
    let bytes = schemas::encode(
        &samples
            .iter()
            .map(SampleInput::publication)
            .collect::<Vec<_>>(),
    )
    .map_err(|error| {
        use ryo_benchmark_contracts::domain::reports::errors::ReportsError;
        match error {
            ReportsError::Invalid(message)
                if message == super::super::constants::INEXACT_RESIDENT =>
            {
                Failure::Inexact
            }
            ReportsError::Invalid(message)
                if message == super::super::constants::MISSING_RESOURCES =>
            {
                Failure::Absent
            }
            ReportsError::Invalid(_)
            | ReportsError::Io(_)
            | ReportsError::Json(_)
            | ReportsError::Toml(_) => Failure::Unexpected,
        }
    })?;
    let actual: serde_json::Value = serde_json::from_slice(&bytes).expect("BMF JSON");
    let expected = serde_json::json!({ "project/ryo/empty-tool-cache": {
        "wall-ms": { "value": 5.0_f64, "lower_value": 2.0_f64, "upper_value": 8.0_f64 },
        "cpu-ms": { "value": 10.0_f64, "lower_value": 4.0_f64, "upper_value": 16.0_f64 },
        "peak-rss-bytes": { "value": 5120.0_f64, "lower_value": 2048.0_f64, "upper_value": 8192.0_f64 }
    }});
    assert_eq!(actual, expected);
    Ok(serde_json::Value::Null)
}
#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, subject);
}
#[test]
fn refuses() {
    run(CASES, Word::Refuses, subject);
}

/// Owned test data, not a certification or measurement proof.
#[derive(Clone)]
struct SampleInput {
    complete_inventory: bool,
    name: SampleName,
    measurement: Measurement,
}
impl SampleInput {
    fn publication(&self) -> PublicationSample<'_> {
        PublicationSample {
            complete_inventory: self.complete_inventory,
            name: &self.name,
            measurement: &self.measurement,
        }
    }
}
