//! Sparse class results of one admitted file population, mergeable without overlap.

use crate::domain::{
    operating::errors::OperatingError,
    positions::types::ClassPlace,
    shared_walk::types::{ClassCount, ClassLease, RowsVisited, prepared_walk::value::PreparedWalk},
};

/// Sparse results for only classes reached in this admitted population.
#[derive(Debug)]
pub(crate) struct NativeBatch {
    class_count: ClassCount,
    refusal: Option<crate::domain::shared_walk::types::WalkRefusal>,
    classes: std::collections::BTreeMap<ClassPlace, ClassLease>,
    counts: crate::domain::shared_walk::types::WalkCounts,
}

impl NativeBatch {
    pub(crate) fn for_program(program: &PreparedWalk) -> Self {
        Self::empty(ClassCount(program.classes().len()))
    }
    /// Preflight the whole transfer before committing any class or counter.
    pub(crate) fn merge(&mut self, next: Self) -> Result<(), OperatingError> {
        if self.class_count != next.class_count
            || next
                .classes
                .keys()
                .any(|class| self.classes.contains_key(class))
        {
            return Err(OperatingError::WrongOperation);
        }
        for (class, result) in next.classes {
            let _previous = self.classes.insert(class, result);
        }
        self.counts.passes.0 = self.counts.passes.0.saturating_add(next.counts.passes.0);
        self.counts.source_rows.0 = self
            .counts
            .source_rows
            .0
            .saturating_add(next.counts.source_rows.0);
        self.counts.predicate_rows.0 = self
            .counts
            .predicate_rows
            .0
            .saturating_add(next.counts.predicate_rows.0);
        self.counts.predicate_tests.0 = self
            .counts
            .predicate_tests
            .0
            .saturating_add(next.counts.predicate_tests.0);
        self.counts.predicate_reuses.0 = self
            .counts
            .predicate_reuses
            .0
            .saturating_add(next.counts.predicate_reuses.0);
        self.counts.decoded_rows.0 = self
            .counts
            .decoded_rows
            .0
            .saturating_add(next.counts.decoded_rows.0);
        self.counts.scratch_classes.0 = self
            .counts
            .scratch_classes
            .0
            .saturating_add(next.counts.scratch_classes.0);
        self.counts.result_slots.0 = self
            .counts
            .result_slots
            .0
            .saturating_add(next.counts.result_slots.0);
        Ok(())
    }
    pub(crate) fn refusal(&self) -> Option<crate::domain::shared_walk::types::WalkRefusal> {
        self.refusal
    }
    pub(in crate::domain::shared_walk) fn refuse_representation(&mut self) {
        self.refusal = Some(crate::domain::shared_walk::types::WalkRefusal::Representation);
    }
    pub(crate) fn counts(&self) -> crate::domain::shared_walk::types::WalkCounts {
        self.counts
    }
    pub(in crate::domain::shared_walk) fn record_pass(&mut self) {
        self.counts.passes.0 = self.counts.passes.0.saturating_add(1);
    }
    pub(in crate::domain::shared_walk) fn record_source_row(&mut self) {
        self.counts.source_rows.0 = self.counts.source_rows.0.saturating_add(1);
    }
    pub(in crate::domain::shared_walk) fn record_predicate_test(&mut self) {
        self.counts.predicate_tests.0 = self.counts.predicate_tests.0.saturating_add(1);
    }
    pub(in crate::domain::shared_walk) fn record_predicate_reuse(&mut self) {
        self.counts.predicate_reuses.0 = self.counts.predicate_reuses.0.saturating_add(1);
    }
    pub(in crate::domain::shared_walk) fn record_decoded_row(&mut self) {
        self.counts.decoded_rows.0 = self.counts.decoded_rows.0.saturating_add(1);
    }
    pub(in crate::domain::shared_walk) fn record_scratch_reset(&mut self) {
        self.counts.scratch_classes.0 = self.counts.scratch_classes.0.saturating_add(1);
    }
    pub(crate) fn class(
        &self,
        place: ClassPlace,
    ) -> Option<&crate::domain::shared_walk::types::ClassResult> {
        self.classes.get(&place).map(|lease| lease.result.as_ref())
    }
    pub(crate) fn lease(&self, place: ClassPlace) -> Option<ClassLease> {
        self.classes.get(&place).map(|lease| ClassLease {
            result: std::sync::Arc::clone(&lease.result),
            visits: lease.visits,
        })
    }
    #[cfg(test)]
    pub(crate) fn visits(&self, place: ClassPlace) -> RowsVisited {
        self.classes
            .get(&place)
            .map_or(RowsVisited::default(), |lease| lease.visits)
    }
    pub(in crate::domain::shared_walk) fn empty(class_count: ClassCount) -> Self {
        Self {
            class_count,
            refusal: None,
            classes: Default::default(),
            counts: Default::default(),
        }
    }
    pub(in crate::domain::shared_walk) fn store(
        &mut self,
        place: ClassPlace,
        result: crate::domain::shared_walk::types::ClassResult,
        visits: RowsVisited,
    ) {
        let _previous = self.classes.insert(
            place,
            ClassLease {
                result: std::sync::Arc::new(result),
                visits,
            },
        );
        self.counts.result_slots.0 = self.counts.result_slots.0.saturating_add(1);
        self.counts.predicate_rows.0 = self.counts.predicate_rows.0.saturating_add(visits.0);
    }
}
