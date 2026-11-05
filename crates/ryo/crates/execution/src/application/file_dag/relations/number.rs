//! Numeric admission witness minted only by the current physical population owner.
use super::*;
use crate::domain::ordered_existence::types::NumberRequirement;

/// The borrowed publication pins its exact population/generation for the invocation.
/// Construction is private: a schema or a caller assertion cannot mint this witness.
pub(in crate::application::file_dag) struct AdmittedNumber<'a> {
    pub(super) publication: &'a PublishedInput,
    _producer: RegionId,
    _population: PopulationId,
    pub(in crate::application::file_dag) column: ryo_values::domain::plan_parts::types::ColumnIndex,
}
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
            .get(&producer.0)
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
impl AdmittedNumber<'_> {
    pub(in crate::application::file_dag) fn publication(&self) -> &PublishedInput {
        self.publication
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::ordered_existence::types::StoredNumberOrigin;
    use ryo_relations::domain::relations::{
        service::kinds,
        types::{ColumnsView, FactColumn},
    };
    use ryo_values::domain::plan_parts::types::{ColumnIndex, StepId};
    #[test]
    fn number_witness_rejects_wrong_producer_and_recycled_generation() {
        let relation = Relation::SourceLocation;
        let schema = kinds(relation);
        let number = ColumnIndex(
            schema
                .iter()
                .position(|kind| *kind == CellKind::Number)
                .unwrap() as u32,
        );
        let columns = schema
            .iter()
            .map(|_| FactColumn(vec![0]))
            .collect::<Vec<_>>();
        let rows = BorrowedRows::Stored(ColumnsView {
            kinds: schema,
            columns: &columns,
        });
        let required = NumberRequirement {
            source: StepId(0),
            column: number,
            width: schema.len(),
            origins: vec![StoredNumberOrigin {
                relation,
                column: number,
            }],
        };
        let owner = RelationOwner::for_request(
            true,
            64,
            (std::sync::Arc::new(
                ryo_derived::inbound::use_cases::derived::use_case::DerivedUseCaseImpl::new(),
            ) as std::sync::Arc<dyn crate::outbound::derived::interface::DerivedPort>),
        );
        let publication = owner.source_borrowed(relation, rows).unwrap();
        owner
            .bind_producer(RegionId(7), publication.as_ref())
            .unwrap();
        assert!(
            owner
                .admit_number(&publication, RegionId(7), &required, rows)
                .unwrap()
                .is_some()
        );
        assert!(
            owner
                .admit_number(&publication, RegionId(8), &required, rows)
                .unwrap()
                .is_none()
        );
        owner.release_population();
        let next = owner.source_borrowed(relation, rows).unwrap();
        owner.bind_producer(RegionId(7), next.as_ref()).unwrap();
        assert!(
            owner
                .admit_number(&next, RegionId(7), &required, rows)
                .unwrap()
                .is_some()
        );
        assert!(
            owner
                .admit_number(&publication, RegionId(7), &required, rows)
                .unwrap()
                .is_none()
        );
    }
}
