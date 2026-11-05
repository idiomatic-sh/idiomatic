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
            .bind_producer(
                super::ProducerScope::File,
                RegionId(7),
                publication.as_ref(),
            )
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
        owner
            .bind_producer(super::ProducerScope::File, RegionId(7), next.as_ref())
            .unwrap();
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
    #[test]
    fn producer_scope_rebinding_refuses_before_registering_an_alias() {
        use crate::application::file_dag::derived::owner::resource_fixture::ResourceFixture;
        use ryo_values::domain::rows::types::Table;

        let fixture = ResourceFixture::new(&[], &[], 0).unwrap();
        let owner = fixture.owner();
        let first = super::ProducerScope::Project(owner.project_identity().unwrap());
        let foreign = super::ProducerScope::Project(owner.project_identity().unwrap());
        let publication = owner.computed(Table::default()).unwrap();
        owner
            .bind_producer(first, RegionId(7), publication.as_ref())
            .unwrap();
        let identity = owner.result_population(publication.as_ref());
        owner
            .bind_producer(first, RegionId(7), publication.as_ref())
            .unwrap();
        assert_eq!(
            owner.bind_producer(foreign, RegionId(8), publication.as_ref()),
            Err(OperatingError::WrongBinding),
        );
        assert_eq!(publication.as_ref().producer_scope.get(), Some(&first));
        assert_eq!(owner.result_population(publication.as_ref()), identity);
        assert!(
            !owner
                .producer_populations
                .borrow()
                .contains_key(&(foreign, 8))
        );
        let independent = owner.computed(Table::default()).unwrap();
        owner
            .bind_producer(foreign, RegionId(8), independent.as_ref())
            .unwrap();
        assert_ne!(owner.result_population(independent.as_ref()), identity);
    }

    #[test]
    fn qualified_producers_release_only_their_actual_population() {
        use crate::application::file_dag::derived::owner::{
            ViewContext,
            graph_fixture::GraphFixture,
            resource_fixture::{ResourceFixture, weak_storage},
        };
        use crate::domain::derived_views::types::{DerivedInputs, DerivedScratch, ViewRequest};
        use ryo_relations::domain::relations::types::{Integer, Value};
        use ryo_values::domain::rows::types::{RowCount, Table};
        // This is a generic owned-row index contract, not a catalog-provider fixture.
        let fixture = ResourceFixture::new(&[Relation::Function], &[], 0).unwrap();
        let owner = fixture.owner();
        let first_scope = super::ProducerScope::Project(owner.project_identity().unwrap());
        let second_scope = super::ProducerScope::Project(owner.project_identity().unwrap());
        let table = |number| Table {
            kinds: vec![CellKind::Number],
            count: RowCount(1),
            cells: vec![Value::Number(Integer(number))],
        };
        let first = owner.computed(table(1)).unwrap();
        let second = owner.computed(table(2)).unwrap();
        owner
            .bind_producer(first_scope, RegionId(7), first.as_ref())
            .unwrap();
        owner
            .bind_producer(second_scope, RegionId(7), second.as_ref())
            .unwrap();
        assert_ne!(
            owner.result_population(first.as_ref()),
            owner.result_population(second.as_ref())
        );
        let acquire = |publication: &PublishedInput| {
            let rows = owner
                .resolve(publication.as_ref(), &|_| Err(OperatingError::NotRead))
                .unwrap();
            let proof = owner
                .admit(publication.as_ref(), BorrowedRows::from(&rows))
                .unwrap();
            let population = proof.identity(owner).unwrap().0;
            let plan = GraphFixture::prepare(
                ViewRequest::Equality {
                    columns: vec![ColumnIndex(0)],
                },
                &rows,
            );
            owner
                .derived
                .acquire_stored(
                    &plan,
                    &[population],
                    ViewContext::default(),
                    DerivedInputs::Rows(&[proof.rows()]),
                    &mut DerivedScratch::default(),
                    None,
                )
                .unwrap()
        };
        owner.pin_derived(first.as_ref());
        owner.pin_derived(second.as_ref());
        let a = acquire(&first);
        let b = acquire(&second);
        let weak_a = weak_storage(&a);
        let weak_b = weak_storage(&b);
        drop((a, b));
        assert!(weak_a.upgrade().is_some());
        assert!(weak_b.upgrade().is_some());
        // Bind/reuse B before A's delayed retirement: release must carry A's key.
        let b = acquire(&second);
        owner.release_derived_producer(first_scope, RegionId(7));
        assert!(weak_a.upgrade().is_none());
        assert!(weak_b.upgrade().is_some());
        drop(b);
        owner.release_derived_producer(second_scope, RegionId(7));
        assert!(weak_b.upgrade().is_none());
        drop((first, second));
        owner.release_population();
        assert_eq!(owner.derived.working_bytes(), 0);
    }
}
