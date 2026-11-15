//! The `native_walk` application operation.

pub(in crate::application::file_dag::program) mod project;

use super::types::Deliveries;
use ryo_execution_domain::domain::shared_walk::types::WalkCounts;
use ryo_execution_domain::domain::{
    shared_walk::types::native_batch::value::NativeBatch, step_classes::types::ClassPlace,
};
use ryo_execution_store::application::file_dag::derived::population::service::PopulationId;
use ryo_relations::domain::relations::types::Relation;
use std::collections::BTreeSet;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub(super) struct WalkSourceIdentity {
    pub(super) population: PopulationId,
    pub(super) relation: Relation,
}

pub(super) struct SourceWork {
    pub(super) ready: BTreeSet<ClassPlace>,
    pub(super) batch: NativeBatch,
}

pub(crate) struct WalkResult {
    lease: ryo_execution_domain::domain::shared_walk::types::ClassLease,
}

impl WalkResult {
    pub(crate) fn result(&self) -> &ryo_execution_domain::domain::shared_walk::types::ClassResult {
        &self.lease.result
    }
    pub(crate) fn visits(&self) -> ryo_execution_domain::domain::shared_walk::types::RowsVisited {
        self.lease.visits
    }
}

/// What one demanded rule still owes the walk before its classes are unneeded.
pub(super) struct Owed {
    pub(super) deliveries: Deliveries,
    pub(super) classes: BTreeSet<ClassPlace>,
}

/// The counts of two walks, added.
pub(super) fn summed(mut total: WalkCounts, counts: WalkCounts) -> WalkCounts {
    total.passes.0 = total.passes.0.saturating_add(counts.passes.0);
    total.source_rows.0 = total.source_rows.0.saturating_add(counts.source_rows.0);
    total.predicate_rows.0 = total
        .predicate_rows
        .0
        .saturating_add(counts.predicate_rows.0);
    total.predicate_tests.0 = total
        .predicate_tests
        .0
        .saturating_add(counts.predicate_tests.0);
    total.predicate_reuses.0 = total
        .predicate_reuses
        .0
        .saturating_add(counts.predicate_reuses.0);
    total.decoded_rows.0 = total.decoded_rows.0.saturating_add(counts.decoded_rows.0);
    total.scratch_classes.0 = total
        .scratch_classes
        .0
        .saturating_add(counts.scratch_classes.0);
    total.result_slots.0 = total.result_slots.0.saturating_add(counts.result_slots.0);
    total
}
