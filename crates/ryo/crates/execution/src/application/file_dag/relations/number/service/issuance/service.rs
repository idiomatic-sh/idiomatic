//! Owner-dependent admission logic lives below its private data representation.
use super::super::*;
use crate::application::file_dag::relations::publication::types::*;
use crate::application::file_dag::relations::*;
use crate::domain::table_model::collections::representations::publication::PublishedRelation;
impl RelationOwner {
    pub(in crate::application::file_dag) fn admit_number<'a>(
        &self,
        publication: &'a PublishedInput,
        producer: RegionId,
        required: &NumberRequirement,
        rows: BorrowedRows<'_>,
    ) -> Result<Option<AdmittedNumber<'a>>, OperatingError> {
        let record = publication.as_ref();
        let expected: Option<(PopulationId, PopulationId)> = self
            .producer_populations
            .borrow()
            .get(&(
                match record.producer_scope.get() {
                    Some(scope) => *scope,
                    None => return Ok(None),
                },
                producer.0,
            ))
            .map(|(population, backing)| (PopulationId(*population), *backing));
        let actual = (
            self.result_population(record),
            self.backing_population(record),
        );
        if !Arc::ptr_eq(&record.generation, &self.generation.borrow()) || expected != Some(actual) {
            return Ok(None);
        }
        // Original-plan lineage permits only copying operators. Every leaf must
        // already have been read and admitted by THIS owner, with total Stored
        // Number decoding and rectangular backing. No additional source read.
        let sources = self.sources.borrow();
        if required.origins.is_empty()
            || required.origins.iter().any(|origin| {
                let Some(source) = sources.get(&origin.relation) else {
                    return true;
                };
                let source = &source.population;
                source.kind != RepresentationKind::Stored
                    || source.kinds.get(origin.column.0 as usize) != Some(&CellKind::Number)
                    || source.storage.len() != source.kinds.len()
                    || source.storage.iter().any(|(_, len)| *len != source.rows)
            })
        {
            return Ok(None);
        }
        drop(sources);
        let shape = self.inspect(record, rows)?;
        // Exact computed payload width is admitted by the owner, not inferred
        // from the query layout. Raw hosted/Rows lineages were already refused.
        if shape != PhysicalShape::Complete
            || rows.kinds().len() != required.width
            || rows.kinds().get(required.column.0 as usize) != Some(&CellKind::Number)
        {
            return Ok(None);
        }
        Ok(Some(AdmittedNumber {
            publication,
            _producer: producer,
            _population: actual.0,
            column: required.column,
        }))
    }
}
