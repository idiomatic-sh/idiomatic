//! Every evidence destination accepts complete samples and refuses incomplete publication.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::outbound::evidence::interface::EvidencePort;
use ryo_benchmark_contracts::domain::measurements::types::Measurement;
use ryo_benchmark_contracts::domain::measurements::types::Resources;
use ryo_benchmark_contracts::domain::measurements::types::{PublicationSample, SampleName};
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
                complete_inventory: false,
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
                complete_inventory:true,
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
                        project_context: None,
                        skipped: Vec::new(),
                        coverage: Some(ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                            selected: 1,
                            evaluated: 1,
                            skipped: 0,
                            unknown_or_unread: 0,
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
                complete_inventory:true,
                name: ryo_benchmark_contracts::domain::measurements::types::SampleName(
                    "project/contestant/cold".into(),
                ),
                measurement: Measurement {
                    wall: Duration::from_millis(5),
                    resources: ryo_benchmark_contracts::domain::measurements::types::ResourceObservation::WallOnly,
                    report: RunReport {
                        execution_metadata:
                            ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete,
                        project_context: None,
                        skipped: Vec::new(),
                        coverage: Some(ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                            selected: 1,
                            evaluated: 1,
                            skipped: 0,
                            unknown_or_unread: 0,
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
        adapter
            .publish(
                &samples
                    .iter()
                    .map(SampleInput::publication)
                    .collect::<Vec<_>>(),
            )
            .map_err(|_invalid_boundary_value| ())
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}

/// Owned test data, not a certification or measurement proof.
#[derive(Clone)]
pub struct SampleInput {
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
