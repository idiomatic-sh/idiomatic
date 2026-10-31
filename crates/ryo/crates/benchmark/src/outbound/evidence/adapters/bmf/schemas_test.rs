//! Independent BMF expectations cover aggregation and absent observations.
use super as schemas;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    diagnostics::types::RunReport,
    measurements::types::{Measurement, Resources, sample::value::BenchmarkSample},
};
use std::time::Duration;

#[derive(Clone, Copy)]
enum Scenario {
    Resources,
    Absent,
    InexactResident,
}
type Input = Scenario;
type Value = serde_json::Value;
#[derive(Debug, PartialEq, Eq)]
enum Failure {
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
        .map(|millis| BenchmarkSample {
            name: crate::domain::measurements::types::SampleName(
                "project/ryo/empty-tool-cache".into(),
            ),
            measurement: Measurement {
                wall: Duration::from_millis(millis),
                resources: if matches!(scenario, Scenario::Absent) {
                    crate::domain::measurements::types::ResourceObservation::WallOnly
                } else {
                    crate::domain::measurements::types::ResourceObservation::Collected(Resources {
                        cpu: Duration::from_millis(millis * 2),
                        peak_rss_bytes: crate::domain::measurements::types::ResidentBytes(
                            if matches!(scenario, Scenario::InexactResident) {
                                super::super::constants::MAX_EXACT_RESIDENT + 1
                            } else {
                                millis * 1024
                            },
                        ),
                    })
                },
                report: RunReport {
                    diagnostics: Vec::new(),
                    files: vec![ryo::linter::RelativePath("input.py".into())],
                },
            },
        })
        .collect();
    let bytes = schemas::encode(&samples).map_err(|error| {
        use crate::domain::diagnostics::errors::DiagnosticsError;
        match error {
            DiagnosticsError::Invalid(message)
                if message == super::super::constants::INEXACT_RESIDENT =>
            {
                Failure::Inexact
            }
            DiagnosticsError::Invalid(message)
                if message == super::super::constants::MISSING_RESOURCES =>
            {
                Failure::Absent
            }
            DiagnosticsError::Invalid(_)
            | DiagnosticsError::Io(_)
            | DiagnosticsError::Json(_)
            | DiagnosticsError::Toml(_) => Failure::Unexpected,
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
