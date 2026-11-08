//! Producer population binding, backing identity and derived-view pinning.
use crate::application::file_dag::relations::*;
use crate::application::file_dag::relations::publication::types::ProducerScope;
impl RelationOwner {
    pub fn project_identity(
        &self,
    ) -> Result<PopulationId, OperatingError> {
        self.allocate_population()
    }
    pub(in crate::application::file_dag::relations) fn allocate_population(
        &self,
    ) -> Result<PopulationId, OperatingError> {
        let id = self.next_population.get();
        self.next_population.set(PopulationId(
            id.0.checked_add(1).ok_or(OperatingError::ShapeMismatch)?,
        ));
        Ok(id)
    }
    /// One canonical DAG producer has one population identity across rules and safe recomputation.
    pub fn bind_producer(
        &self,
        scope: ProducerScope,
        node: RegionId,
        result: &RelationResult,
    ) -> Result<(), OperatingError> {
        self.generation(result)?;
        match result.producer_scope.get() {
            Some(existing) if *existing != scope => return Err(OperatingError::WrongBinding),
            Some(_) => {}
            None => result
                .producer_scope
                .set(scope)
                .map_err(|_| OperatingError::WrongBinding)?,
        }
        if matches!(result.body, RelationBody::Source(_)) || result.producer.get().is_some() {
            self.producer_populations
                .borrow_mut()
                .entry(ProducerKey {
                    scope,
                    region: node,
                })
                .or_insert(ProducerPopulation {
                    population: self.result_population(result),
                    backing: self.backing_population(result),
                });
            return Ok(());
        }
        let mut known = self.producer_populations.borrow_mut();
        let known = known
            .entry(ProducerKey {
                scope,
                region: node,
            })
            .or_insert(ProducerPopulation {
                population: result.population,
                backing: self.backing_population(result),
            });
        result
            .producer
            .set(known.population)
            .map_err(|_| OperatingError::WrongBinding)?;
        Ok(())
    }
    pub(in crate::application::file_dag::relations) fn result_population(&self, result: &RelationResult) -> PopulationId {
        *result.producer.get().unwrap_or(&result.population)
    }
    pub(in crate::application::file_dag::relations) fn backing_relation(result: &RelationResult) -> Option<Relation> {
        match &result.body {
            RelationBody::Source(source) => Some(source.relation),
            RelationBody::Computed(_) | RelationBody::Compact(_) => None,
        }
    }
    pub(in crate::application::file_dag::relations) fn backing_population(&self, result: &RelationResult) -> PopulationId {
        match &result.body {
            RelationBody::Source(source) => {
                self.sources.borrow().get(&source.relation).map_or_else(
                    || self.result_population(result),
                    |source| source.canonical_population,
                )
            }
            RelationBody::Computed(_) | RelationBody::Compact(_) => self.result_population(result),
        }
    }
    pub fn pin_derived(&self, result: &RelationResult) {
        let population = self.result_population(result);
        self.derived.pin(population);
        let backing = self.backing_population(result);
        if backing != population {
            self.derived.pin(backing);
        }
    }
    /// Read-only evidence for the actual producer in the actual population.
    #[cfg(test)]
    pub fn producer_view_witness(
        &self,
        scope: ProducerScope,
        node: RegionId,
        kind: ryo_derived::domain::request::types::DerivedKind,
    ) -> Result<Option<crate::application::file_dag::derived::owner::witness::service::BackingWitness>, OperatingError>
    {
        let Some(known) = self
            .producer_populations
            .borrow()
            .get(&ProducerKey {
                scope,
                region: node,
            })
            .copied()
        else {
            return Ok(None);
        };
        self.derived
            .population_view_witness(&[known.population, known.backing], kind)
    }
    /// Global consumer cancellation need not retain the producer payload.
    /// A generation clear has already released all pins and removed this map.
    pub fn release_derived_producer(
        &self,
        scope: ProducerScope,
        node: ryo_execution_domain::domain::executable_program::types::RegionId,
    ) {
        if let Some(known) = self
            .producer_populations
            .borrow()
            .get(&ProducerKey {
                scope,
                region: node,
            })
            .copied()
        {
            self.derived.release(known.population);
            if known.backing != known.population {
                self.derived.release(known.backing);
            }
        }
    }
}
