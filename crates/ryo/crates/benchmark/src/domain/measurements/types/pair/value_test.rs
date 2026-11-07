//! Naming preserves each original certified observation and contestant order.
use super::value::Pair;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::measurements::types::Measurement;
use ryo_benchmark_contracts::domain::measurements::types::ResourceObservation;
use ryo_benchmark_contracts::domain::measurements::types::SampleName;

const CASES: &[Case<(), [u128; 2], ()>] = &[Case {
    name: CaseName("naming preserves both observations"),
    word: Word::Succeeds,
    input: || (),
    expect: Expect::Value([3, 7]),
}];

#[test]
fn succeeds() {
    run(CASES, Word::Succeeds, |()| {
        let observation = |milliseconds| Measurement {
            wall: std::time::Duration::from_millis(milliseconds),
            resources: ResourceObservation::WallOnly,
            report: ryo_benchmark_contracts::domain::reports::types::RunReport {
                execution_metadata:
                    ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete,
                project_context: None,
                files: vec![],
                skipped: Vec::new(),
                coverage: Some(
                    ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                        selected: 1,
                        evaluated: 1,
                        skipped: 0,
                        unknown_or_unread: 0,
                    },
                ),
                diagnostics: vec![],
            },
        };
        let pair = Pair {
            eligibility:
                crate::domain::measurements::types::MeasurementEligibility::CompleteInventory,
            reference: observation(3),
            candidate: observation(7),
        };
        assert_eq!(pair.reference().wall.as_millis(), 3);
        assert_eq!(pair.candidate().wall.as_millis(), 7);
        let [reference, candidate] =
            pair.samples([SampleName(String::new()), SampleName(String::new())]);
        assert!(reference.name().0.is_empty());
        assert!(candidate.name().0.is_empty());
        Ok([
            reference.measurement().wall.as_millis(),
            candidate.measurement().wall.as_millis(),
        ])
    });
}
