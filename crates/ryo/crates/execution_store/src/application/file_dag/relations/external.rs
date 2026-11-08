//! Explicit demanded source/external admission, never an internal-publication fallback.
use super::*;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
impl RelationOwner {
    #[cfg(any(test, feature = "fixtures"))]
    pub(crate) fn admit_relation<'a>(
        &self,
        relation: Relation,
        rows: RepresentationInput<'a>,
    ) -> Result<BorrowedInput<'a>, OperatingError> {
        let record = self.source_borrowed(relation, rows)?;
        self.admit(record.as_ref(), rows)
    }
}
