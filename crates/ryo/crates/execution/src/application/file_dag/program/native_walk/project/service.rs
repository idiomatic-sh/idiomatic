//! One admitted project-stage source population is scanned once for every rule of its cohort.
use super::super::lease::service::PermittedWalk;
use super::super::{
    Deliveries, Fate, FileWalk, Owed, SourceWork, Submission, WalkDelivery, WalkResult,
    WalkSourceIdentity,
};
use crate::application::file_dag::program::admission::AdmittedInput;
use crate::application::file_dag::relations::RelationOwner;
use std::rc::Rc;
use crate::application::file_dag::executor::{
    KernelCall, PrimitiveExecutor,
    KernelAnswer, KernelFault,
};
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::runtime::service::{KernelPermit, ReadySelection};
use crate::domain::{
    coverage::types::RuleIndex,
    operating::{errors::OperatingError, types::OperatorContext},
    step_classes::types::ClassPlace,
    shared_walk::types::{WalkCounts, WalkWork, native_batch::value::NativeBatch},
    table_model::types::Rows,
};
use ryo_relations::domain::relations::types::Relation;
use ryo_values::domain::plan_parts::types::StepScope;
use std::{
    cell::{Cell, Ref, RefCell},
    collections::{BTreeSet, HashMap},
};

/// The production invocation is explicitly bound to a file owner or project stage.
/// An invocation of either partition cannot opt out of its shared-walk owner.
pub(in crate::application::file_dag::program) enum WalkPartition<'a> {
    File(&'a FileWalk),
    Project(&'a ProjectWalk),
}
impl WalkPartition<'_> {
    pub(in crate::application::file_dag::program) fn validate(&self, permit: &KernelPermit<'_>) -> Result<(), OperatingError> {
        match self {
            Self::File(walk) => permit.require_file_walk(&walk.generation),
            Self::Project(_) => permit.require_project_walk(),
        }
    }
    pub(in crate::application::file_dag::program) fn submit<'facts>(
        &self,
        prepared: &PreparedProgram,
        ready: &ReadySelection<'_, '_>,
        inputs: &[AdmittedInput],
        owner: &Rc<RelationOwner>,
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
            tracing::trace!(target: crate::application::file_dag::program::constants::WALK_TARGET,
                event = crate::application::file_dag::program::constants::WALK_REFUSED,
                logical = permit.descriptor().logical().0, relation = ?relation,
                generation = ?permit.diagnostic_generation(), population = ?input.population(owner),
                classes = ?[class.0],
                reason = ?crate::domain::shared_walk::types::WalkRefusal::Representation);
        }
        Ok(submitted)
    }
    pub(in crate::application::file_dag::program) fn acquire(
        &self,
        prepared: &PreparedProgram,
        permit: &KernelPermit<'_>,
        rows: &Rows<'_>,
        delivery: WalkDelivery,
        context: &mut OperatorContext<'_>,
        kernels: &dyn PrimitiveExecutor,
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
/// The batches of one publication scope. A batch is keyed by the admitted source population
/// and its relation; the scope is the generation, so no key spans two scopes.
pub(in crate::application::file_dag::program) struct ProjectWalk {
    // A batch lives while a declared rule that consumes its relation still owes a delivery,
    // then retires; a lease handed out earlier keeps its own result.
    batches: RefCell<HashMap<WalkSourceIdentity, SourceWork>>,
    // The declared roots of the cohorts in flight: no head is admitted without its declaration.
    owed: RefCell<HashMap<RuleIndex, Owed>>,
    // Counters of retired batches, so the scope's totals survive retirement.
    retired: Cell<WalkCounts>,
    work: RefCell<WalkWork>,
}
impl ProjectWalk {
    pub(in crate::application::file_dag::program) fn new() -> Self {
        Self {
            batches: RefCell::new(HashMap::new()),
            owed: RefCell::new(HashMap::new()),
            retired: Cell::new(WalkCounts::default()),
            work: RefCell::new(WalkWork::default()),
        }
    }
    /// Only a project-stage class admitted for a declared root joins this walk.
    pub(in crate::application::file_dag::program) fn admit(
        &self,
        prepared: &PreparedProgram,
        class: ClassPlace,
    ) -> Result<(), OperatingError> {
        let class_scope = prepared
            .walk()
            .classes()
            .get(class.0)
            .map(|class| class.scope);
        if class_scope != Some(StepScope::CrossFile) {
            return Err(OperatingError::WrongOperation);
        }
        if !self
            .owed
            .borrow()
            .values()
            .any(|owed| owed.classes.contains(&class))
        {
            return Err(OperatingError::NotRead);
        }
        Ok(())
    }
    /// Called only when the existing runtime has completed the head's demand program.
    pub(in crate::application::file_dag::program) fn submit(
        &self,
        prepared: &PreparedProgram,
        ready: &ReadySelection<'_, '_>,
        relation: Relation,
        rows: &Rows<'_>,
    ) -> Result<Submission, OperatingError> {
        let permit = ready.permit();
        let Some(class) = prepared.walk_class(permit.descriptor()) else {
            return Ok(Submission::Declined);
        };
        permit.validate_program(prepared.program().as_ref())?;
        self.admit(prepared, class)?;
        let Some(admitted) = permit.admit_walk_source(relation, rows)? else {
            return Ok(Submission::Declined);
        };
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
    pub(in crate::application::file_dag::program) fn prepare<'walk>(
        &'walk self,
        prepared: &PreparedProgram,
        permit: &KernelPermit<'_>,
        relation: Relation,
        rows: &Rows<'_>,
        context: &mut OperatorContext<'_>,
        kernels: &dyn PrimitiveExecutor,
    ) -> Result<Option<Ref<'walk, NativeBatch>>, OperatingError> {
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
            source.ready.iter().copied().collect::<Vec<_>>()
        };
        if !ready.is_empty() {
            let mut work = self.work.borrow_mut();
            permit.validate_walk_source(&admitted)?;
            let request =
                PermittedWalk::new(permit, prepared, admitted, context, &ready, &mut work);
            let batch = match kernels.execute(KernelCall::Walk { request }) {
                Ok(KernelAnswer::Walk(batch)) => batch,
                Err(KernelFault::Refused(error)) => return Err(error),
                Err(KernelFault::Portable(error)) => {
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
    /// Declare that `rule` is a root of a cohort and will make `deliveries` entry-point
    /// calls that may consume this scope's project-stage classes.
    pub(in crate::application::file_dag::program) fn owe(
        &self,
        prepared: &PreparedProgram,
        rule: RuleIndex,
        deliveries: Deliveries,
    ) {
        let classes = prepared
            .walk()
            .class_of(rule)
            .into_iter()
            .flatten()
            .flatten()
            .copied()
            .filter(|class| {
                prepared
                    .walk()
                    .classes()
                    .get(class.0)
                    .is_some_and(|class| class.scope == StepScope::CrossFile)
            })
            .collect::<BTreeSet<_>>();
        if deliveries.0 == 0 || classes.is_empty() {
            return;
        }
        // A rule submitted twice owes each submission's delivery.
        let mut owed = self.owed.borrow_mut();
        match owed.get_mut(&rule) {
            Some(entry) => {
                entry.deliveries = Deliveries(entry.deliveries.0.saturating_add(deliveries.0));
            }
            None => {
                owed.insert(
                    rule,
                    Owed {
                        deliveries,
                        classes,
                    },
                );
            }
        }
    }
    /// One entry-point call of `rule` finished, whatever its outcome (success,
    /// cancellation or error). The last one releases the rule's classes and every
    /// batch no remaining declared rule can still consume.
    pub(in crate::application::file_dag::program) fn delivered(
        &self,
        prepared: &PreparedProgram,
        rule: RuleIndex,
    ) {
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
    /// The cohort evaluation is over: nothing can lease from any batch again.
    pub(in crate::application::file_dag::program) fn retire_all(&self) {
        self.owed.borrow_mut().clear();
        self.retire(|_, _| Fate::Release);
    }
    fn retire(&self, release: impl Fn(&WalkSourceIdentity, &SourceWork) -> Fate) {
        let mut retired = self.retired.get();
        self.batches.borrow_mut().retain(|identity, source| {
            if matches!(release(identity, source), Fate::Keep) {
                return true;
            }
            retired = FileWalk::sum(retired, source.batch.counts());
            false
        });
        self.retired.set(retired);
    }
    #[cfg(test)]
    pub(in crate::application::file_dag::program) fn retained(
        &self,
    ) -> super::super::RetainedBatches {
        super::super::RetainedBatches(self.batches.borrow().len())
    }
    #[cfg(test)]
    pub(in crate::application::file_dag::program) fn counts(&self) -> WalkCounts {
        self.batches
            .borrow()
            .values()
            .fold(self.retired.get(), |total, source| {
                FileWalk::sum(total, source.batch.counts())
            })
    }
}
