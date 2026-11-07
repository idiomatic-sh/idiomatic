//! One admitted source population is scanned only after ordinary input admission.
pub(crate) mod lease;
pub(super) mod project;
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::runtime::service::KernelPermit;
use crate::domain::{coverage::types::RuleIndex, shared_walk::types::WalkCounts};
use crate::{
    application::file_dag::derived::population::types::PopulationId,
    domain::{
        operating::{errors::OperatingError, types::OperatorContext},
        positions::types::ClassPlace,
        shared_walk::types::{WalkWork, native_batch::value::NativeBatch},
        table_model::types::Rows,
    },
};
use lease::service::PermittedWalk;

/// Whether the admitted source joined the shared walk.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Submission {
    Submitted,
    Declined,
}
use ryo_relations::domain::relations::types::Relation;
use std::{
    cell::{Cell, Ref, RefCell},
    collections::{BTreeSet, HashMap},
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct WalkSourceIdentity {
    population: PopulationId,
    relation: Relation,
}

struct SourceWork {
    ready: BTreeSet<ClassPlace>,
    batch: NativeBatch,
}

/// The production invocation is explicitly bound to a file owner or project stage.
/// An invocation of either partition cannot opt out of its shared-walk owner.
pub(super) enum WalkPartition<'a> {
    File(&'a FileWalk),
    Project(&'a project::service::ProjectWalk),
}
impl WalkPartition<'_> {
    pub(super) fn validate(&self, permit: &KernelPermit<'_>) -> Result<(), OperatingError> {
        match self {
            Self::File(walk) => permit.require_file_walk(&walk.generation),
            Self::Project(_) => permit.require_project_walk(),
        }
    }
    pub(super) fn submit<'facts>(
        &self,
        prepared: &PreparedProgram,
        ready: &crate::application::file_dag::program::runtime::service::ReadySelection<'_, '_>,
        inputs: &[super::admission::AdmittedInput],
        owner: &std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
        read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
    ) -> Result<Submission, OperatingError> {
        self.validate(ready.permit())?;
        let permit = ready.permit();
        let Some(class) = prepared.walk_class(permit.descriptor()) else {
            return Ok(Submission::Declined);
        };
        let [input] = inputs else {
            return Err(OperatingError::WrongOperation);
        };
        let rows = input.rows(owner, read)?;
        let relation = relation_of(prepared, permit)?;
        let submitted = match self {
            Self::File(walk) => walk.submit(prepared, ready, relation, &rows)?,
            Self::Project(walk) => walk.submit(prepared, ready, relation, &rows)?,
        };
        if submitted == Submission::Declined {
            tracing::trace!(target: super::constants::WALK_TARGET,
                event = super::constants::WALK_REFUSED,
                logical = permit.descriptor().logical.0, relation = ?relation,
                generation = ?permit.diagnostic_generation(), population = ?input.population(owner),
                classes = ?[class.0],
                reason = ?crate::domain::shared_walk::types::WalkRefusal::Representation);
        }
        Ok(submitted)
    }
    pub(super) fn acquire(
        &self,
        prepared: &PreparedProgram,
        permit: &KernelPermit<'_>,
        rows: &Rows<'_>,
        delivery: WalkDelivery,
        context: &mut OperatorContext<'_>,
        kernels: &dyn super::super::executor::PrimitiveExecutor,
    ) -> Result<Option<WalkResult>, OperatingError> {
        self.validate(permit)?;
        let Some(class) = prepared.walk_class(permit.descriptor()) else {
            return Ok(None);
        };
        match (delivery, rows) {
            (WalkDelivery::Positions, Rows::Tree(_))
            | (WalkDelivery::Stored, Rows::Stored(_))
            | (WalkDelivery::Rows, _) => {}
            _ => return Ok(None),
        }
        let relation = relation_of(prepared, permit)?;
        let prepared_batch = match self {
            Self::File(walk) => walk.prepare(prepared, permit, relation, rows, context, kernels)?,
            Self::Project(walk) => {
                walk.prepare(prepared, permit, relation, rows, context, kernels)?
            }
        };
        let Some(batch) = prepared_batch else {
            return Ok(None);
        };
        // Result leases contain no RefCell guard or provider borrow. Reentrant
        // delivery may safely advance another admitted source in this file.
        Ok(batch.lease(class).map(|lease| WalkResult { lease }))
    }
}
fn relation_of(
    prepared: &PreparedProgram,
    permit: &KernelPermit<'_>,
) -> Result<Relation, OperatingError> {
    let class = prepared
        .walk_class(permit.descriptor())
        .ok_or(OperatingError::WrongOperation)?;
    prepared
        .walk()
        .classes()
        .get(class.0)
        .map(|class| class.source)
        .ok_or(OperatingError::WrongOperation)
}
/// Physical delivery must decline before opening any shared predicate work.
#[derive(Clone, Copy)]
pub(super) enum WalkDelivery {
    Positions,
    Stored,
    Rows,
}
pub(super) struct WalkResult {
    lease: crate::domain::shared_walk::types::ClassLease,
}
impl WalkResult {
    pub(super) fn result(&self) -> &crate::domain::shared_walk::types::ClassResult {
        &self.lease.result
    }
    pub(super) fn visits(&self) -> crate::domain::shared_walk::types::RowsVisited {
        self.lease.visits
    }
}

/// Entry-point calls a rule has yet to make.
#[derive(Clone, Copy, PartialEq, Eq)]
pub(super) struct Deliveries(pub(super) usize);

/// What retirement decides for one batch.
enum Fate {
    Keep,
    Release,
}
/// Batches a file walk still holds.
#[cfg(test)]
#[derive(Debug, PartialEq, Eq)]
pub(super) struct RetainedBatches(pub(super) usize);

/// What one demanded rule still owes the walk before its classes are unneeded.
struct Owed {
    deliveries: Deliveries,
    classes: BTreeSet<ClassPlace>,
}

pub(super) struct FileWalk {
    generation: super::admission::FileGeneration,
    // A source batch lives while a rule that consumes its relation still owes a
    // delivery, then retires; a lease handed out earlier keeps its own result.
    batches: RefCell<HashMap<WalkSourceIdentity, SourceWork>>,
    owed: RefCell<HashMap<RuleIndex, Owed>>,
    // Counters of retired batches, so the file's totals survive retirement.
    retired: Cell<WalkCounts>,
    work: RefCell<WalkWork>,
}
impl FileWalk {
    pub(super) fn new(generation: super::admission::FileGeneration) -> Self {
        Self {
            generation,
            batches: RefCell::new(HashMap::new()),
            owed: RefCell::new(HashMap::new()),
            retired: Cell::new(WalkCounts::default()),
            work: RefCell::new(WalkWork::default()),
        }
    }
    /// Called only when the existing runtime has completed the head's demand program.
    /// A rule being requested is never sufficient to queue its other classes.
    fn submit(
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

    fn prepare<'walk>(
        &'walk self,
        prepared: &PreparedProgram,
        permit: &KernelPermit<'_>,
        relation: Relation,
        rows: &Rows<'_>,
        context: &mut OperatorContext<'_>,
        kernels: &dyn super::super::executor::PrimitiveExecutor,
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
            let batch = match kernels.execute(super::super::executor::KernelCall::Walk { request })
            {
                Ok(super::super::executor::KernelAnswer::Walk(batch)) => batch,
                Err(super::super::executor::KernelFault::Refused(error)) => {
                    return Err(error);
                }
                Err(super::super::executor::KernelFault::Portable(error)) => {
                    return Err(crate::domain::operator::contract::types::portable_fault(
                        error,
                    ));
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
    pub(super) fn owe(&self, prepared: &PreparedProgram, rule: RuleIndex, deliveries: Deliveries) {
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
    pub(super) fn delivered(&self, prepared: &PreparedProgram, rule: RuleIndex) {
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
    pub(super) fn retire_all(&self) {
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
    pub(super) fn lease_any(&self, class: ClassPlace) -> Option<WalkResult> {
        self.batches
            .borrow()
            .values()
            .find_map(|source| source.batch.lease(class))
            .map(|lease| WalkResult { lease })
    }
    #[cfg(test)]
    pub(super) fn retained(&self) -> RetainedBatches {
        RetainedBatches(self.batches.borrow().len())
    }
    fn sum(mut total: WalkCounts, counts: WalkCounts) -> WalkCounts {
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
    pub(super) fn counts(&self) -> WalkCounts {
        self.batches
            .borrow()
            .values()
            .fold(self.retired.get(), |total, source| {
                Self::sum(total, source.batch.counts())
            })
    }
}
