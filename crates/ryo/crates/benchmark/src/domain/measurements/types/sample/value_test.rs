//! Read-only access preserves the issued observation when cloned.
use super::value::BenchmarkSample;
use crate::cases::{Case, CaseName, Expect, Word, run};
use ryo_benchmark_contracts::domain::measurements::types::Measurement;
use ryo_benchmark_contracts::domain::measurements::types::ResourceObservation;
use ryo_benchmark_contracts::domain::measurements::types::SampleName;

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
            eligibility:
                crate::domain::measurements::types::MeasurementEligibility::CompleteInventory,
            name: SampleName(String::new()),
            measurement: Measurement {
                wall: std::time::Duration::from_millis(11),
                resources: ResourceObservation::WallOnly,
                report: ryo_benchmark_contracts::domain::reports::types::RunReport {
                    execution_metadata:
                        ryo_benchmark_contracts::domain::reports::types::ExecutionMetadata::Complete,
                    project_context:
                        ryo_benchmark_contracts::domain::reports::types::ProjectContext::NotRequested,
                    files: vec![],
                    skipped: Vec::new(),
                    coverage: ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Accounted(
                        ryo_benchmark_contracts::domain::reports::types::RuleCoverage {
                            selected: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
                            evaluated: ryo_benchmark_contracts::domain::reports::types::PairCount(1),
                            skipped: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                            unknown_or_unread: ryo_benchmark_contracts::domain::reports::types::PairCount(0),
                        },
                    ),
                    diagnostics: vec![],
                },
            },
        };
        let projected = original.publication();
        assert_eq!(
            projected.inventory,
            ryo_benchmark_contracts::domain::measurements::types::InventoryStatus::Complete
        );
        assert!(std::ptr::eq(projected.measurement, original.measurement()));
        assert!(std::ptr::eq(projected.name, original.name()));
        let cloned = original.clone();
        assert_eq!(cloned.name(), original.name());
        Ok(cloned.measurement().wall.as_millis())
    });
}
