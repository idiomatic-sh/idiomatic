//! One cell per family and mismatch: the issued capability binds the owner generation, the
//! exact plan, the admitted population and the issued descriptor.
use ryo_derived::domain::request::types::DerivedKind;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_store::application::file_dag::derived::owner::acquisition::family::fixtures::{
    Scenario, exercise,
};

#[test]
fn every_family_refuses_every_mismatched_capability() {
    let refusals = [
        (Scenario::Stale, OperatingError::NotRead),
        (Scenario::OtherPlan, OperatingError::WrongOperation),
        (Scenario::OtherPopulation, OperatingError::NotRead),
        (
            Scenario::RetargetedDescriptor,
            OperatingError::WrongOperation,
        ),
    ];
    for kind in DerivedKind::ALL.iter().copied() {
        for (scenario, expected) in &refusals {
            assert_eq!(
                exercise(kind, *scenario).err().as_ref(),
                Some(expected),
                "{kind:?} {scenario:?}"
            );
        }
    }
}
