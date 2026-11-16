//! Independent BMF expectations cover aggregation and absent observations.
use super as schemas;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::measurements::types::Measurement;
use ryo_benchmark_contracts::domain::measurements::types::Resources;
use ryo_benchmark_contracts::domain::measurements::types::{PublicationSample, SampleName};
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use crate::outbound::evidence::adapters::bmf::types::MeasureDocument;
use crate::outbound::evidence::fixtures::sample;
use std::time::Duration;

#[derive(Clone, Copy)]
enum Scenario {
    Resources,
    Absent,
    InexactResident,
    ParityRules,
    ParityMetrics,
    /// The costs of django, as their names and values.
    Costs,
    /// The verification times of F401.
    Verification,
    /// The growth of one shape.
    Growth,
    /// The same project added twice.
    DuplicateProject,
    /// A document nothing was added to.
    Empty,
}
type Input = Scenario;
type Value = serde_json::Value;
type Failure = Refusal;
#[derive(Debug, PartialEq, Eq)]
enum Refusal {
    Absent,
    Inexact,
    Duplicate,
    Empty,
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
        name: CaseName(
            "the per-rule document names each contestant, says an agreeing rule by its count and is indented by one space",
        ),
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
    Case {
        name: CaseName(
            "only ryo's cost of django is a measure, under python/django, each condition under its own name",
        ),
        word: Word::Succeeds,
        input: || Scenario::Costs,
        expect: Expect::Value(serde_json::Value::Null),
    },
    Case {
        name: CaseName("verification times are the bench's measures, under python/verify/<rule>"),
        word: Word::Succeeds,
        input: || Scenario::Verification,
        expect: Expect::Value(serde_json::Value::Null),
    },
    Case {
        name: CaseName("a shape's growth is the bench's measure, under python/growth/<shape>"),
        word: Word::Succeeds,
        input: || Scenario::Growth,
        expect: Expect::Value(serde_json::Value::Null),
    },
    Case {
        name: CaseName("a project added twice is refused"),
        word: Word::Refuses,
        input: || Scenario::DuplicateProject,
        expect: Expect::Error(Refusal::Duplicate),
    },
    Case {
        name: CaseName("a document nothing was added to is refused"),
        word: Word::Refuses,
        input: || Scenario::Empty,
        expect: Expect::Error(Refusal::Empty),
    },
];
fn subject(scenario: Input) -> Result<serde_json::Value, Failure> {
    if matches!(
        scenario,
        Scenario::Costs
            | Scenario::Verification
            | Scenario::Growth
            | Scenario::DuplicateProject
            | Scenario::Empty
    ) {
        return document(scenario);
    }
    let parity = match scenario {
        Scenario::ParityRules => Some((schemas::parity_rules(&publication()), PER_RULE)),
        Scenario::ParityMetrics => Some((
            schemas::parity_document(&std::collections::BTreeMap::from([schemas::parity_counts(
                &publication(),
            )])),
            PARITY_METRICS,
        )),
        Scenario::Resources
        | Scenario::Absent
        | Scenario::InexactResident
        | Scenario::Costs
        | Scenario::Verification
        | Scenario::Growth
        | Scenario::DuplicateProject
        | Scenario::Empty => None,
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

/// The per-rule document of a fixture parity: `B` agrees, so it is its count in `identical`; `A` keeps its row under `rules`.
const PER_RULE: &str = r#"{
 "identical": {"B":0},
 "rules": [
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
  }
 ]
}"#;

/// The parity metrics of the fixture parity.
const PARITY_METRICS: &str =
    r#"{"ruff":{"differing-rules":{"value":1.0},"unaccepted-differences":{"value":2.0}}}"#;

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

/// A measure of the document as `[value, lower, upper]`.
fn observed(value: f64, lower: f64, upper: f64) -> serde_json::Value {
    serde_json::json!({ "value": value, "lower_value": lower, "upper_value": upper })
}

/// Milliseconds of a duration, as the document writes them.
fn millis(milliseconds: u64) -> f64 {
    Duration::from_millis(milliseconds).as_secs_f64() * 1000.0
}

/// Seconds of a duration of that many milliseconds, as the document writes them.
fn secs(milliseconds: u64) -> f64 {
    Duration::from_millis(milliseconds).as_secs_f64()
}

/// The expected document of each succeeding scenario, compared in full by the subject.
fn expected(scenario: Scenario) -> serde_json::Value {
    match scenario {
        Scenario::Costs => serde_json::json!({
            "python/django": {
                "cpu-s": observed(secs(1_001), secs(1_000), secs(1_002)),
                "wall-s": observed(secs(1_501), secs(1_500), secs(1_502)),
                "peak-rss-bytes": observed(1_000_001.0, 1_000_000.0, 1_000_002.0),
                "warm-cpu-s": observed(secs(21), secs(20), secs(22)),
                "warm-peak-rss-bytes": observed(200_001.0, 200_000.0, 200_002.0),
                "edited-cpu-s": observed(secs(61), secs(60), secs(62)),
                "edited-peak-rss-bytes": observed(300_001.0, 300_000.0, 300_002.0),
                "store-bytes": observed(30.0, 30.0, 30.0),
                "store-files": observed(3.0, 3.0, 3.0),
            }
        }),
        Scenario::Verification => serde_json::json!({
            "python/verify/F401": {
                "total-ms": observed(millis(40), millis(40), millis(40)),
                "max-case-ms": observed(millis(9), millis(9), millis(9)),
                "compile-ms": observed(millis(3), millis(3), millis(3)),
            }
        }),
        Scenario::Growth => serde_json::json!({
            "python/growth/F401/module/assign": {
                "doubling-ratio": observed(1.9, 1.9, 1.9),
                "larger-ms": observed(millis(120), millis(120), millis(120)),
            }
        }),
        Scenario::DuplicateProject
        | Scenario::Empty
        | Scenario::Resources
        | Scenario::Absent
        | Scenario::InexactResident
        | Scenario::ParityRules
        | Scenario::ParityMetrics => serde_json::Value::Null,
    }
}

/// The one document of a bench run built from the scenario, compared in full with what it should hold.
fn document(scenario: Input) -> Result<Value, Failure> {
    let language = "python".into();
    let mut document = MeasureDocument::default();
    let added = match scenario {
        Scenario::Costs => schemas::costs(&mut document, &language, &sample::costs()),
        Scenario::Verification => schemas::verification(&mut document, &language, &sample::timings()),
        Scenario::Growth => schemas::growth(&mut document, &language, &sample::growth()),
        Scenario::DuplicateProject => schemas::costs(&mut document, &language, &sample::costs())
            .and_then(|()| schemas::costs(&mut document, &language, &sample::costs())),
        Scenario::Empty
        | Scenario::Resources
        | Scenario::Absent
        | Scenario::InexactResident
        | Scenario::ParityRules
        | Scenario::ParityMetrics => Ok(()),
    };
    added.map_err(|error| {
        if error.to_string().contains("twice") {
            Refusal::Duplicate
        } else {
            Refusal::Unexpected
        }
    })?;
    let bytes = schemas::encode_document(&document).map_err(|_empty| Refusal::Empty)?;
    let written: serde_json::Value =
        serde_json::from_slice(&bytes).map_err(|_malformed| Refusal::Unexpected)?;
    // The whole document is compared: a reference benchmark, a missing measure or a value under another condition's name
    // makes it differ.
    if written == expected(scenario) {
        Ok(serde_json::Value::Null)
    } else {
        Ok(written)
    }
}
