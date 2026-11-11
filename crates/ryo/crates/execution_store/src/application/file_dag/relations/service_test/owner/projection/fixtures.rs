//! The support of the `projection` cases.
use ryo_execution_domain::domain::table_model::types::PublishedRelation;

#[derive(Debug)]
pub(super) struct Forged;

impl PublishedRelation for Forged {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }
}
