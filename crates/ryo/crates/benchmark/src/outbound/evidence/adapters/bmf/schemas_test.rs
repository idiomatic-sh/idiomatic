//! Independent BMF expectations cover aggregation and absent observations.
use super as schemas;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    diagnostics::types::RunReport,
    measurements::types::{Measurement, Resources, sample::value::BenchmarkSample},
};
use std::time::Duration;

type Input = bool;
type Value = serde_json::Value;
type Failure = bool;

const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("median and bounds preserve independent metric scales"),
        word: Word::Succeeds,
        input: || true,
        expect: Expect::Value(serde_json::Value::Null),
    },
    Case {
        name: CaseName("wall only sample cannot masquerade as full evidence"),
        word: Word::Refuses,
        input: || false,
        expect: Expect::Error(true),
    },
];
fn subject(resources: bool) -> Result<serde_json::Value, bool> {
    let samples: Vec<_> = [2, 8, 4, 6]
        .into_iter()
        .map(|millis| BenchmarkSample {
            name: crate::domain::measurements::types::SampleName(
                "project/ryo/empty-tool-cache".into(),
            ),
            measurement: Measurement {
                wall: Duration::from_millis(millis),
                resources: if resources {
                    crate::domain::measurements::types::ResourceObservation::Collected(Resources {
                        cpu: Duration::from_millis(millis * 2),
                        peak_rss_bytes: crate::domain::measurements::types::ResidentBytes(
                            millis * 1024,
                        ),
                    })
                } else {
                    crate::domain::measurements::types::ResourceObservation::WallOnly
                },
                report: RunReport {
                    diagnostics: Vec::new(),
                    files: vec![ryo::linter::RelativePath("input.py".into())],
                },
            },
        })
        .collect();
    let bytes = schemas::encode(&samples).map_err(|_| true)?;
    let actual: serde_json::Value = serde_json::from_slice(&bytes).expect("BMF JSON");
    let expected = serde_json::json!({ "project/ryo/empty-tool-cache": {
        "wall-ms": { "value": 5.0, "lower_value": 2.0, "upper_value": 8.0 },
        "cpu-ms": { "value": 10.0, "lower_value": 4.0, "upper_value": 16.0 },
        "peak-rss-bytes": { "value": 5120.0, "lower_value": 2048.0, "upper_value": 8192.0 }
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
