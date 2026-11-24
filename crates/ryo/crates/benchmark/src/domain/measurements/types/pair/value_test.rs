//! A pair keeps each certified observation, in contestant order, and its eligibility.
use super::value::Pair;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::measurements::types::Measurement;
use ryo_benchmark_contracts::domain::measurements::types::ResourceObservation;

const CASES: &[Case<(), [u128; 2], ()>] = &[Case {
    name: CaseName("the pair keeps both observations and its eligibility"),
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
            phases: None,
            report: ryo_benchmark_contracts::domain::reports::types::RunReport {
                execution_metadata:
                    ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete,
                project_context:
                    ryo_benchmark_contracts::domain::reports::types::ProjectContext::NotRequested,
                files: vec![],
                skipped: Vec::new(),
                coverage:
                    ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Accounted(
                        ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                            selected: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
                            evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(
                                1,
                            ),
                            skipped: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                            unknown_or_unread:
                                ryo_benchmark_contracts::domain::reports::types::PairCount(0),
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
        assert_eq!(
            pair.eligibility(),
            crate::domain::measurements::types::MeasurementEligibility::CompleteInventory
        );
        Ok([
            pair.reference().wall.as_millis(),
            pair.candidate().wall.as_millis(),
        ])
    });
}
