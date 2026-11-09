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
    ParityRules,
    ParityMetrics,
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
    Case {
        name: CaseName("the per-rule document names each contestant and is indented by one space"),
        word: Word::Succeeds,
        input: || Scenario::ParityRules,
        expect: Expect::Value(serde_json::Value::Null),
    },
    Case {
        name: CaseName("the parity metrics count differing rules and unaccepted differences"),
        word: Word::Succeeds,
        input: || Scenario::ParityMetrics,
        expect: Expect::Value(serde_json::Value::Null),
    },
];
fn subject(scenario: Input) -> Result<serde_json::Value, Failure> {
    let parity = match scenario {
        Scenario::ParityRules => Some((schemas::parity_rules(&publication()), PER_RULE)),
        Scenario::ParityMetrics => Some((schemas::parity_metrics(&publication()), PARITY_METRICS)),
        Scenario::Resources | Scenario::Absent | Scenario::InexactResident => None,
    };
    if let Some((bytes, expected)) = parity {
        let text = String::from_utf8(bytes.map_err(|_refused| Failure::Unexpected)?)
            .map_err(|_invalid| Failure::Unexpected)?;
        assert_eq!(text, expected);
        return Ok(serde_json::Value::Null);
    }
    let samples: Vec<_> = [2, 8, 4, 6]
        .into_iter()
        .map(|millis| SampleInput {
            inventory: ryo_benchmark_contracts::domain::measurements::types::InventoryStatus::Complete,
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
                    project_context: ryo_benchmark_contracts::domain::reports::types::ProjectContext::NotRequested,
                    skipped: Vec::new(),
                    coverage: ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Unaccounted,
                    diagnostics: Vec::new(),
                    files: vec![ryo::linter::RelativePath("input.py".into())],
                },
            },
        })
        .collect();
    let observations = samples
        .iter()
        .map(SampleInput::publication)
        .collect::<Vec<_>>();
    let publication =
        crate::outbound::evidence::types::measured_publication::value::MeasuredPublication::checked(
            &observations,
        );
    let bytes = schemas::encode(&publication).map_err(|error| {
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
    inventory: ryo_benchmark_contracts::domain::measurements::types::InventoryStatus,
    name: SampleName,
    measurement: Measurement,
}
impl SampleInput {
    fn publication(&self) -> PublicationSample<'_> {
        PublicationSample {
            inventory: self.inventory,
            name: &self.name,
            measurement: &self.measurement,
        }
    }
}

/// The per-rule document of a fixture parity, as `json.dumps(rows, indent=1)` spells it.
const PER_RULE: &str = r#"[
 {
  "rule": "A",
  "ryo": 2,
  "ruff": 1,
  "matched": 1,
  "missing": 0,
  "extra": 1,
  "end_differs": 1,
  "missing_samples": [],
  "extra_samples": [
   {
    "file": "a.py",
    "line": 1,
    "column": 1,
    "end": [
     1,
     3
    ]
   }
  ],
  "end_differs_samples": [
   {
    "file": "a.py",
    "line": 2,
    "column": 5,
    "ryo_end": [
     2,
     7
    ],
    "ruff_end": [
     2,
     9
    ]
   }
  ],
  "class": "ryo-extra",
  "differences": 1
 },
 {
  "rule": "B",
  "ryo": 0,
  "ruff": 0,
  "matched": 0,
  "missing": 0,
  "extra": 0,
  "end_differs": 0,
  "missing_samples": [],
  "extra_samples": [],
  "end_differs_samples": [],
  "class": "identical",
  "differences": 0
 }
]"#;

/// The parity metrics of the fixture parity.
const PARITY_METRICS: &str = r#"{"ruff":{"differing-rules":{"value":1.0},"unaccepted-differences":{"value":2.0}}}"#;

/// A parity with one rule differing by an extra finding and an end, and one identical rule.
fn publication() -> ryo_benchmark_contracts::domain::reports::types::ParityReport<'static> {
    use ryo_benchmark_contracts::domain::reports::types::ParityProblem;
    use ryo_benchmark_contracts::domain::reports::types::{
        ContestantName, EndPair, FindingCount, FindingPosition, LocatedFinding, ParityReport,
        RuleComparison, SourceColumn, SourceLine,
    };
    let at = |line, column| FindingPosition {
        line: SourceLine(line),
        column: SourceColumn(column),
    };
    let located = |start, end| LocatedFinding {
        file: ryo::linter::RelativePath("a.py".into()),
        start,
        end,
    };
    let rule = |name: &str| ryo::linter::RuleId::try_from(name).expect("fixture rule");
    ParityReport {
        reference: ContestantName("ruff"),
        candidate: ContestantName("ryo"),
        rules: vec![
            RuleComparison {
                rule: rule("A"),
                matched: FindingCount(1),
                missing: Vec::new(),
                extra: vec![located(at(1, 1), at(1, 3))],
                end_differences: vec![EndPair {
                    candidate: located(at(2, 5), at(2, 7)),
                    reference: located(at(2, 5), at(2, 9)),
                }],
            },
            RuleComparison {
                rule: rule("B"),
                matched: FindingCount(0),
                missing: Vec::new(),
                extra: Vec::new(),
                end_differences: Vec::new(),
            },
        ],
        problems: vec![ParityProblem("one".into()), ParityProblem("two".into())],
    }
}
