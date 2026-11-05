//! Borrow a producer-proven selection after reacquiring its declared source.
use super::*;
impl RelationOwner {
    #[cfg(test)]
    pub(in crate::application::file_dag) fn selected_rows<'f>(
        &self,
        publication: &RelationResult,
        source_rows: ryo_relations::domain::relations::types::ColumnsView<'f>,
    ) -> Result<Rows<'f>, OperatingError> {
        self.generation(publication)?;
        let RelationBody::Selected { source, selection } = &publication.body else {
            return Err(OperatingError::WrongOperation);
        };
        self.generation(source)?;
        if !self.matches_current(source, BorrowedRows::Stored(source_rows)) {
            return Err(OperatingError::NotRead);
        }
        Ok(Rows::Selected(SelectedRows {
            source: source_rows,
            selection: Arc::clone(selection),
        }))
    }
    pub(in crate::application::file_dag) fn stored_equality(
        &self,
        plan: &Arc<crate::domain::derived_views::types::DerivedContract>,
        relation: Relation,
        source: ryo_relations::domain::relations::types::ColumnsView<'_>,
    ) -> Result<super::super::derived::view::AcquiredView, OperatingError> {
        use crate::domain::derived_views::types::{DerivedInputs, DerivedScratch};
        let proof = self.admit_relation(relation, BorrowedRows::Stored(source))?;
        let (population, _) = proof.identity(self)?;
        self.derived.acquire_stored_with_route(
            crate::application::file_dag::route_counts::BuildRoute::Column,
            plan,
            &[population],
            Default::default(),
            DerivedInputs::Rows(&[proof.rows()]),
            &mut DerivedScratch::default(),
            Some((relation, source)),
        )
    }
}
