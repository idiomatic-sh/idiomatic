//! Deliver one owner-produced selection without rereading or retaining a provider borrow.
use super::*;
impl RelationOwner {
    pub(in crate::application::file_dag) fn deliver_selection<'f>(
        &self,
        produced: ProducedSelection<'f>,
    ) -> Result<(PublishedInput, Rows<'f>), OperatingError> {
        let rows = Rows::Selected(SelectedRows {
            source: produced.rows,
            selection: Arc::clone(&produced.selection),
        });
        let publication = self.publish_selection(produced)?;
        Ok((publication, rows))
    }
}
