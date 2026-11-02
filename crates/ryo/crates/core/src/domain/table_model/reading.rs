//! Pure borrowed relation reader port.
use crate::domain::{operating::errors::OperatingError, table_model::types::Rows};
use ryo_language::domain::relations::types::Relation;
/// What gives the rows of a relation, where the store keeps them.
pub(crate) trait Reading<'f> {
    /// The rows of the relation.
    ///
    /// # Errors
    ///
    /// `NotRead` when nothing provides them.
    fn rows(&self, relation: Relation) -> Result<Rows<'f>, OperatingError>;
}

impl<'f, F: Fn(Relation) -> Result<Rows<'f>, OperatingError>> Reading<'f> for F {
    fn rows(&self, relation: Relation) -> Result<Rows<'f>, OperatingError> {
        self(relation)
    }
}
