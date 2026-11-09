//! Every evidence destination accepts complete samples and refuses incomplete publication.
use crate::cases::{Case, CaseName, Expect, Word, run};
use super::sample::SampleInput;
use crate::outbound::evidence::interface::EvidencePort;
use ryo_benchmark_contracts::domain::measurements::types::Measurement;
use ryo_benchmark_contracts::domain::measurements::types::Resources;
use ryo_benchmark_contracts::domain::measurements::types::SampleName;
use ryo_benchmark_contracts::domain::reports::types::RunReport;
use std::time::Duration;

type Input = Vec<SampleInput>;
type Value = ();
type Failure = ();

/// Independent port observations shared by concrete and fake implementations.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("diagnostic pair cannot publish benchmark metrics"),
        word: Word::Refuses,
        input: || {
            let samples = (CASES[1].input)();
            let original = samples[0].measurement.clone();
            vec![SampleInput {
                inventory: ryo_benchmark_contracts::domain::measurements::types::InventoryStatus::Diagnostic,
                name: SampleName("diagnostic/reference".into()),
                measurement: original,
            }]
        },
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("complete observation publishes"),
        word: Word::Succeeds,
        input: || {
            vec![SampleInput {
                inventory: ryo_benchmark_contracts::domain::measurements::types::InventoryStatus::Complete,
                name: ryo_benchmark_contracts::domain::measurements::types::SampleName(
                    "project/contestant/cold".into(),
                ),
                measurement: Measurement {
                    wall: Duration::from_millis(5),
                    resources: ryo_benchmark_contracts::domain::measurements::types::ResourceObservation::Collected(
                        Resources {
                            cpu: Duration::from_millis(4),
                            peak_rss_bytes: ryo_benchmark_contracts::domain::measurements::types::ResidentBytes(4096),
                        },
                    ),
                    report: RunReport {
                        execution_metadata:
                            ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete,
                        project_context: ryo_benchmark_contracts::domain::reports::types::ProjectContext::NotRequested,
                        skipped: Vec::new(),
                        coverage: ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Accounted(ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                            selected: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
                            evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
                            skipped: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                            unknown_or_unread: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                        }),
                        diagnostics: vec![],
                        files: vec![ryo::linter::RelativePath("input".into())],
                    },
                },
            }]
        },
        expect: Expect::Value(()),
    },
    Case {
        name: CaseName("empty publication is refused"),
        word: Word::Refuses,
        input: Vec::new,
        expect: Expect::Error(()),
    },
    Case {
        name: CaseName("missing resources are refused"),
        word: Word::Refuses,
        input: || {
            vec![SampleInput {
                inventory: ryo_benchmark_contracts::domain::measurements::types::InventoryStatus::Complete,
                name: ryo_benchmark_contracts::domain::measurements::types::SampleName(
                    "project/contestant/cold".into(),
                ),
                measurement: Measurement {
                    wall: Duration::from_millis(5),
                    resources: ryo_benchmark_contracts::domain::measurements::types::ResourceObservation::WallOnly,
                    report: RunReport {
                        execution_metadata:
                            ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete,
                        project_context: ryo_benchmark_contracts::domain::reports::types::ProjectContext::NotRequested,
                        skipped: Vec::new(),
                        coverage: ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Accounted(ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                            selected: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
                            evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
                            skipped: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                            unknown_or_unread: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                        }),
                        diagnostics: vec![],
                        files: vec![ryo::linter::RelativePath("input".into())],
                    },
                },
            }]
        },
        expect: Expect::Error(()),
    },
];

/// Run publication observations against one supplied destination.
/// # Panics
/// Panics if the destination violates the publication contract.
pub fn check(adapter: &impl EvidencePort) {
    let subject = |samples: Vec<SampleInput>| {
        let observations = samples
            .iter()
            .map(SampleInput::publication)
            .collect::<Vec<_>>();
        let publication =
            crate::outbound::evidence::types::measured_publication::value::MeasuredPublication::checked(
                &observations,
            );
        adapter
            .publish(&publication)
            .map_err(|_invalid_boundary_value| ())
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
    let report = (CASES[1].input)()
        .into_iter()
        .map(|sample| sample.measurement.report)
        .next()
        .expect("the complete observation row holds one sample");
    assert_eq!(
        adapter
            .comparison(
                &report,
                &report,
                ryo_benchmark_contracts::domain::reports::types::ReportComparison {
                    diagnostics: ryo_benchmark_contracts::domain::reports::types::Comparison::Exact,
                    execution_metadata:
                        ryo_benchmark_contracts::domain::reports::types::Comparison::Exact,
                },
                ryo_benchmark_contracts::domain::reports::types::ComparisonContract::Diagnostics,
            )
            .map_err(|_invalid_boundary_value| ()),
        Ok(()),
        "an exact comparison of two complete reports is written"
    );
    assert_eq!(
        adapter
            .parity(&ryo_benchmark_contracts::domain::reports::types::ParityReport {
                reference: ryo_benchmark_contracts::domain::reports::types::ContestantName("reference"),
                candidate: ryo_benchmark_contracts::domain::reports::types::ContestantName("candidate"),
                rules: Vec::new(),
                problems: Vec::new(),
            })
            .map_err(|_invalid_boundary_value| ()),
        Ok(()),
        "a parity report is written"
    );
}
