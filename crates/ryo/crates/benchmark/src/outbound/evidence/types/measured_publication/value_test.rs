//! A destination receives exactly the borrowed observations authorized by the workflow.
use super::value::MeasuredPublication;
use ryo_benchmark_contracts::domain::measurements::types::{
    Measurement, PublicationSample, ResourceObservation, SampleName,
};
use ryo_benchmark_contracts::domain::reports::types::{ExecutionMetadata, RunReport};
#[test]
fn payload_view_retains_exact_nonempty_slice_and_observation_identity() {
    let first = Measurement {
        wall: std::time::Duration::from_millis(1),
        resources: ResourceObservation::WallOnly,
        report: RunReport {
            execution_metadata: ExecutionMetadata::Unknown,
            project_context: ryo_benchmark_contracts::domain::reports::types::ProjectContext::NotRequested,
            diagnostics: vec![],
            skipped: vec![],
            coverage: ryo_benchmark_contracts::domain::reports::types::CoverageAccounting::Unaccounted,
            files: vec![],
        },
    };
    let second = Measurement {
        wall: std::time::Duration::from_millis(9),
        ..first.clone()
    };
    let names = [SampleName("first".into()), SampleName("second".into())];
    let observations = [
        PublicationSample {
            inventory: ryo_benchmark_contracts::domain::measurements::types::InventoryStatus::Complete,
            name: &names[0],
            measurement: &first,
        },
        PublicationSample {
            inventory: ryo_benchmark_contracts::domain::measurements::types::InventoryStatus::Complete,
            name: &names[1],
            measurement: &second,
        },
    ];
    let publication = MeasuredPublication::checked(&observations);
    assert!(std::ptr::eq(publication.samples(), observations.as_slice()));
    assert_eq!(publication.samples().len(), 2);
    assert!(std::ptr::eq(publication.samples()[0].measurement, &first));
    assert!(std::ptr::eq(publication.samples()[1].measurement, &second));
    assert_eq!(
        publication.samples()[0].measurement.wall,
        std::time::Duration::from_millis(1)
    );
    assert_eq!(
        publication.samples()[1].measurement.wall,
        std::time::Duration::from_millis(9)
    );
}
