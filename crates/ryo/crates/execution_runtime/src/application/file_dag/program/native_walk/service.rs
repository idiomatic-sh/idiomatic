//! The `native_walk` application operation.

use super::super::runtime::types::Submission;
use super::lease::service::PermittedWalk;
#[cfg(test)]
use super::types::RetainedBatches;
use super::types::{Deliveries, Fate};
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::runtime::service::KernelPermit;
use rustc_hash::FxHashMap;
use ryo_execution_domain::domain::{coverage::types::RuleIndex, shared_walk::types::WalkCounts};
use ryo_execution_domain::domain::{
    operating::errors::OperatingError,
    operator::types::OperatorContext,
    shared_walk::types::{WalkWork, native_batch::value::NativeBatch},
    step_classes::types::ClassPlace,
    table_model::types::Rows,
};
use ryo_execution_store::application::file_dag::derived::population::service::PopulationId;
use ryo_relations::domain::relations::types::Relation;
use std::{
    cell::{Cell, Ref, RefCell},
    collections::BTreeSet,
};

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
    pub(super) lease: ryo_execution_domain::domain::shared_walk::types::ClassLease,
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

pub(crate) struct FileWalk {
    pub(super) generation: super::super::admission::service::FileGeneration,
    // A source batch lives while a rule that consumes its relation still owes a
    // delivery, then retires; a lease handed out earlier keeps its own result.
    batches: RefCell<FxHashMap<WalkSourceIdentity, SourceWork>>,
    owed: RefCell<FxHashMap<RuleIndex, Owed>>,
    // Counters of retired batches, so the file's totals survive retirement.
    retired: Cell<WalkCounts>,
    work: RefCell<WalkWork>,
}

impl FileWalk {
    pub(crate) fn new(generation: super::super::admission::service::FileGeneration) -> Self {
        Self {
            generation,
            batches: RefCell::new(FxHashMap::default()),
            owed: RefCell::new(FxHashMap::default()),
            retired: Cell::new(WalkCounts::default()),
            work: RefCell::new(WalkWork::default()),
        }
    }
    /// Called only when the existing runtime has completed the head's demand program.
    /// A rule being requested is never sufficient to queue its other classes.
    pub(super) fn submit(
        &self,
        prepared: &PreparedProgram,
        ready: &crate::application::file_dag::program::runtime::service::ReadySelection<'_, '_>,
        relation: Relation,
        rows: &Rows<'_>,
    ) -> Result<Submission, OperatingError> {
        let permit = ready.permit();
        let Some(class) = prepared.walk_class(permit.descriptor()) else {
            return Ok(Submission::Declined);
        };
        permit.validate_program(prepared.program().as_ref())?;
        let Some(admitted) = permit.admit_walk_source(relation, rows)? else {
            return Ok(Submission::Declined);
        };
        if matches!(rows, Rows::Tree(tree) if tree.file != self.generation.file()) {
            return Err(OperatingError::NotRead);
        }
        let key = WalkSourceIdentity {
            population: admitted.population(),
            relation,
        };
        let mut sources = self.batches.borrow_mut();
        let source = sources.entry(key).or_insert_with(|| SourceWork {
            ready: BTreeSet::new(),
            batch: NativeBatch::for_program(prepared.walk()),
        });
        if source.batch.class(class).is_none() {
            source.ready.extend([class]);
        }
        Ok(Submission::Submitted)
    }

    pub(super) fn prepare<'walk>(
        &'walk self,
        prepared: &PreparedProgram,
        permit: &KernelPermit<'_>,
        relation: Relation,
        rows: &Rows<'_>,
        context: &mut OperatorContext<'_>,
        kernels: &dyn super::super::super::executor::service::PrimitiveExecutor,
    ) -> Result<Option<Ref<'walk, NativeBatch>>, OperatingError> {
        if let Rows::Tree(tree) = rows {
            if tree.file != self.generation.file() {
                return Err(OperatingError::NotRead);
            }
        }
        // Revalidate even a cache hit: population alone does not establish the
        // relation, current physical view, generation, or live invocation.
        permit.validate_program(prepared.program().as_ref())?;
        let Some(admitted) = permit.admit_walk_source(relation, rows)? else {
            return Ok(None);
        };
        let identity = WalkSourceIdentity {
            population: admitted.population(),
            relation: admitted.relation(),
        };
        let ready = {
            let sources = self.batches.borrow();
            let source = sources.get(&identity).ok_or(OperatingError::NotRun)?;
            // Commit queue removal only after a complete batch is accepted.
            // A transport/permit failure must not silently discard other waiters.
            source.ready.iter().copied().collect::<Vec<_>>()
        };
        if !ready.is_empty() {
            let mut work = self.work.borrow_mut();
            permit.validate_walk_source(&admitted)?;
            let request =
                PermittedWalk::new(permit, prepared, admitted, context, &ready, &mut work);
            let batch = match kernels
                .execute(super::super::super::executor::types::KernelCall::Walk { request })
            {
                Ok(super::super::super::executor::types::KernelAnswer::Walk(batch)) => batch,
                Err(super::super::super::executor::types::KernelFault::Refused(error)) => {
                    return Err(error);
                }
                Err(super::super::super::executor::types::KernelFault::Portable(error)) => {
                    return Err(
                        ryo_execution_domain::domain::operating::service::portable_fault(error),
                    );
                }
                _ => return Err(OperatingError::WrongOperation),
            };
            let mut sources = self.batches.borrow_mut();
            let source = sources.get_mut(&identity).ok_or(OperatingError::NotRun)?;
            source.batch.merge(batch)?;
            source.ready.retain(|held| !ready.contains(held));
        }
        Ref::filter_map(self.batches.borrow(), |batches| {
            batches.get(&identity).map(|source| &source.batch)
        })
        .map(Some)
        .map_err(|_| OperatingError::NotRun)
    }
    /// Record that `rule` will make `deliveries` entry-point calls that may
    /// consume this file's shared classes. Rules without a walk class owe nothing.
    pub(crate) fn owe(&self, prepared: &PreparedProgram, rule: RuleIndex, deliveries: Deliveries) {
        let classes = prepared
            .walk()
            .class_of(rule)
            .into_iter()
            .flatten()
            .flatten()
            .copied()
            .collect::<BTreeSet<_>>();
        if deliveries.0 > 0 && !classes.is_empty() {
            self.owed.borrow_mut().extend([(
                rule,
                Owed {
                    deliveries,
                    classes,
                },
            )]);
        }
    }
    /// One entry-point call of `rule` finished, whatever its outcome (success,
    /// cancellation or error). The last one releases the rule's classes and every
    /// batch no remaining rule can still consume.
    pub(crate) fn delivered(&self, prepared: &PreparedProgram, rule: RuleIndex) {
        {
            let mut owed = self.owed.borrow_mut();
            let Some(entry) = owed.get_mut(&rule) else {
                return;
            };
            entry.deliveries = Deliveries(entry.deliveries.0.saturating_sub(1));
            if entry.deliveries.0 > 0 {
                return;
            }
            drop(owed.remove(&rule));
        }
        let pending = self
            .owed
            .borrow()
            .values()
            .flat_map(|owed| owed.classes.iter())
            .filter_map(|class| prepared.walk().classes().get(class.0))
            .map(|class| class.source)
            .collect::<Vec<_>>();
        self.retire(|identity, source| {
            if source.ready.is_empty() && !pending.contains(&identity.relation) {
                Fate::Release
            } else {
                Fate::Keep
            }
        });
    }
    /// The file owner is done: nothing can lease from any batch again.
    pub(crate) fn retire_all(&self) {
        self.owed.borrow_mut().clear();
        self.retire(|_, _| Fate::Release);
    }
    fn retire(&self, release: impl Fn(&WalkSourceIdentity, &SourceWork) -> Fate) {
        let mut retired = self.retired.get();
        self.batches.borrow_mut().retain(|identity, source| {
            if matches!(release(identity, source), Fate::Keep) {
                return true;
            }
            retired = Self::sum(retired, source.batch.counts());
            false
        });
        self.retired.set(retired);
    }
    /// Lease a class from whichever retained batch holds it, as `acquire` would.
    #[cfg(test)]
    pub(crate) fn lease_any(&self, class: ClassPlace) -> Option<WalkResult> {
        self.batches
            .borrow()
            .values()
            .find_map(|source| source.batch.lease(class))
            .map(|lease| WalkResult { lease })
    }
    #[cfg(test)]
    pub(crate) fn retained(&self) -> RetainedBatches {
        RetainedBatches(self.batches.borrow().len())
    }
    pub(super) fn sum(mut total: WalkCounts, counts: WalkCounts) -> WalkCounts {
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
    #[cfg(any(test, feature = "fixtures"))]
    pub(crate) fn counts(&self) -> WalkCounts {
        self.batches
            .borrow()
            .values()
            .fold(self.retired.get(), |total, source| {
                Self::sum(total, source.batch.counts())
            })
    }
}
