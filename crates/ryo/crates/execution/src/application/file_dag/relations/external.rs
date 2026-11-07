//! Explicit demanded source/external admission, never an internal-publication fallback.
use super::*;
impl RelationOwner {
    pub(in crate::application::file_dag) fn admit_relation<'a>(
        &self,
        relation: Relation,
        rows: BorrowedRows<'a>,
    ) -> Result<BorrowedInput<'a>, OperatingError> {
        let record = self.source_borrowed(relation, rows)?;
        self.admit(record.as_ref(), rows)
    }
}
