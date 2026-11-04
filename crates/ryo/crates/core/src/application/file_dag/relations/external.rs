//! Explicit demanded source/external admission, never an internal-publication fallback.
use super::*;
impl RelationOwner {
    pub(in crate::application::file_dag) fn admit_relation<'a>(
        &self,
        relation: Relation,
        rows: BorrowedRows<'a>,
    ) -> Result<AdmittedInput<'a>, OperatingError> {
        let record = self.source_borrowed(relation, rows)?;
        self.admit(record.as_ref(), rows)
    }
    pub(in crate::application::file_dag) fn admit_external<'a>(
        &self,
        rows: BorrowedRows<'a>,
    ) -> Result<AdmittedInput<'a>, OperatingError> {
        rows.validate()?;
        let population = PopulationId(self.allocate_population()?);
        Ok(AdmittedInput {
            backing_population: population,
            backing_relation: None,
            canonical_backing: false,
            rows,
            shape: PhysicalShape::Complete,
            origin: (population, None, Arc::clone(&self.generation.borrow())),
        })
    }
}
