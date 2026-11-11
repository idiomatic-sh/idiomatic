//! Forged metadata never gains authority it was not issued.
use super::super::super::super::RelationOwner;
use super::super::fixtures::columns;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
use ryo_execution_domain::domain::{
    operating::errors::OperatingError,
    table_model::types::{PublishedRelation, Rows},
};
use ryo_relations::domain::relations::types::{CellKind, ColumnsView};
#[derive(Debug)]
struct Forged;
impl PublishedRelation for Forged {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
#[test]
fn a_foreign_publication_with_correct_kind_has_no_admission_authority() {
    let owner = RelationOwner::new(
        crate::application::file_dag::relations::types::ReuseEligibility::Eligible,
        std::sync::Arc::new(crate::application::file_dag::fixtures::derived()),
    );
    let physical = columns(&[CellKind::Number]);
    let rows = Rows::Stored(ColumnsView {
        kinds: &[CellKind::Number],
        columns: &physical,
    });
    assert!(matches!(
        owner.inspect(&Forged, RepresentationInput::from(&rows)),
        Err(OperatingError::NotRead)
    ));
    assert!(matches!(
        owner.admit(&Forged, RepresentationInput::from(&rows)),
        Err(OperatingError::NotRead)
    ));
    assert!(matches!(
        owner.resolve(&Forged, &|_| panic!(
            "forged publication must not acquire a provider"
        )),
        Err(OperatingError::NotRead)
    ));
    assert_eq!(
        owner.checks.get(),
        crate::application::file_dag::relations::types::CheckCount(0)
    );
}
