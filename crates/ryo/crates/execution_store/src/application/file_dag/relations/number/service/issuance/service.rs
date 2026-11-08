//! Owner-dependent admission logic lives below its private data representation.
use super::super::*;
use crate::application::file_dag::relations::publication::types::*;
use crate::application::file_dag::relations::*;
use ryo_execution_domain::domain::table_model::types::RepresentationInput;
impl RelationOwner {
    /// Admits a numeric source for the producer that requires it, the one way to mint the witness.
    pub fn admit_number<'a>(
        &self,
        publication: &'a PublishedInput,
        producer: RegionId,
        required: &NumberRequirement,
        rows: RepresentationInput<'_>,
    ) -> Result<Option<AdmittedNumber<'a>>, OperatingError> {
        let record = publication.as_ref();
        let scope = match record.producer_scope.get() {
            Some(scope) => *scope,
            None => return Ok(None),
        };
        let expected = self
            .producer_populations
            .borrow()
            .get(&ProducerKey {
                scope,
                region: producer,
            })
            .copied();
        let actual = ProducerPopulation {
            population: self.result_population(record),
            backing: self.backing_population(record),
        };
        if self.matches_generation(&record.generation) == Sameness::Different || expected != Some(actual) {
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
                    || source.storage.iter().any(|span| span.length.0 != source.rows.0)
            })
        {
            return Ok(None);
        }
        drop(sources);
        let shape = self.inspect(record, rows)?;
        // Exact computed payload width is admitted by the owner, not inferred
        // from the query layout. Raw hosted/Rows lineages were already refused.
        if shape != PhysicalShape::Complete
            || rows.kinds().len() != required.width.0
            || rows.kinds().get(required.column.0 as usize) != Some(&CellKind::Number)
        {
            return Ok(None);
        }
        Ok(Some(AdmittedNumber {
            publication,
            column: required.column,
        }))
    }
}
