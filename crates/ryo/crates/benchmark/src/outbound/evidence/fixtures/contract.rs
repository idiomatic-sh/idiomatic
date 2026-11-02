//! Every evidence destination accepts complete samples and refuses incomplete publication.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    diagnostics::types::RunReport,
    measurements::types::{Measurement, Resources, sample::value::BenchmarkSample},
};
use crate::outbound::evidence::interface::EvidencePort;
use std::time::Duration;

type Input = Vec<BenchmarkSample>;
type Value = ();
type Failure = ();

/// Independent port observations shared by concrete and fake implementations.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("complete observation publishes"),
        word: Word::Succeeds,
        input: || {
            vec![BenchmarkSample {
                name: crate::domain::measurements::types::SampleName(
                    "project/contestant/cold".into(),
                ),
                measurement: Measurement {
                    wall: Duration::from_millis(5),
                    resources: crate::domain::measurements::types::ResourceObservation::Collected(
                        Resources {
                            cpu: Duration::from_millis(4),
                            peak_rss_bytes: crate::domain::measurements::types::ResidentBytes(4096),
                        },
                    ),
                    report: RunReport {
                        execution_metadata:
                            crate::domain::diagnostics::types::ExecutionMetadata::Complete,
                        project_context: None,
                        skipped: Vec::new(),
                        coverage: Some(crate::domain::diagnostics::types::RuleCoverage {
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
            vec![BenchmarkSample {
                name: crate::domain::measurements::types::SampleName(
                    "project/contestant/cold".into(),
                ),
                measurement: Measurement {
                    wall: Duration::from_millis(5),
                    resources: crate::domain::measurements::types::ResourceObservation::WallOnly,
                    report: RunReport {
                        execution_metadata:
                            crate::domain::diagnostics::types::ExecutionMetadata::Complete,
                        project_context: None,
                        skipped: Vec::new(),
                        coverage: Some(crate::domain::diagnostics::types::RuleCoverage {
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
    let subject = |samples: Vec<BenchmarkSample>| {
        adapter
            .publish(&samples)
            .map_err(|_invalid_boundary_value| ())
    };
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
