//! Every evidence destination accepts complete samples and refuses incomplete publication.
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::{
    diagnostics::types::RunReport,
    measurements::types::{Measurement, Resources, Sample},
};
use crate::outbound::evidence::interface::EvidencePort;
use std::time::Duration;

type Input = Vec<Sample>;
type Value = ();
type Failure = ();

/// Independent port observations shared by concrete and fake implementations.
pub const CASES: &[Case<Input, Value, Failure>] = &[
    Case {
        name: CaseName("complete observation publishes"),
        word: Word::Succeeds,
        input: || {
            vec![Sample {
                name: "project/contestant/cold".into(),
                measurement: Measurement {
                    wall: Duration::from_millis(5),
                    resources: Some(Resources {
                        cpu: Duration::from_millis(4),
                        peak_rss_bytes: 4096,
                    }),
                    report: RunReport {
                        diagnostics: vec![],
                        files: vec!["input".into()],
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
            vec![Sample {
                name: "project/contestant/cold".into(),
                measurement: Measurement {
                    wall: Duration::from_millis(5),
                    resources: None,
                    report: RunReport {
                        diagnostics: vec![],
                        files: vec!["input".into()],
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
    let subject = |samples: Vec<Sample>| adapter.publish(&samples).map_err(|_| ());
    run(CASES, Word::Succeeds, subject);
    run(CASES, Word::Refuses, subject);
}
