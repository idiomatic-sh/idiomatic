//! Read-only access preserves the issued observation when cloned.
use super::value::BenchmarkSample;
use crate::cases::{Case, CaseName, Expect, Word, run};
use crate::domain::measurements::types::{Measurement, ResourceObservation, SampleName};

const CASES: &[Case<(), u128, ()>] = &[Case {
    name: CaseName("cloning retains the original observation"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value(11),
}];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |()| {
        let original = BenchmarkSample {
            name: SampleName(String::new()),
            measurement: Measurement {
                wall: std::time::Duration::from_millis(11),
                resources: ResourceObservation::WallOnly,
                report: crate::domain::diagnostics::types::RunReport {
                    execution_metadata:
                        crate::domain::diagnostics::types::ExecutionMetadata::Complete,
                    project_context: None,
                    files: vec![],
                    skipped: Vec::new(),
                    coverage: Some(crate::domain::diagnostics::types::RuleCoverage {
                        selected: 1,
                        evaluated: 1,
                        skipped: 0,
                        unknown_or_unread: 0,
                    }),
                    diagnostics: vec![],
                },
            },
        };
        let cloned = original.clone();
        assert_eq!(cloned.name(), original.name());
        Ok(cloned.measurement().wall.as_millis())
    });
}
