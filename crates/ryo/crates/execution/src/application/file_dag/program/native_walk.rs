//! One admitted source population is scanned only after ordinary input admission.
use crate::application::file_dag::program::preparation::types::PreparedProgram;
use crate::application::file_dag::program::runtime::types::KernelPermit;
use crate::{
    application::file_dag::derived::owner::PopulationId,
    domain::{
        operating::{errors::OperatingError, types::OperatorContext},
        positions::types::ClassPlace,
        shared_walk::types::{WalkWork, prepared_walk::value::NativeBatch},
        table_model::types::Rows,
    },
};
use ryo_relations::domain::relations::types::Relation;
use std::{
    cell::{Ref, RefCell},
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
/// A file invocation cannot opt out of the shared-walk owner.
pub(super) enum WalkPartition<'a> {
    File(&'a FileWalk),
    Project,
}
impl WalkPartition<'_> {
    pub(super) fn validate(&self, permit: &KernelPermit<'_>) -> Result<(), OperatingError> {
        match self {
            Self::File(walk) => permit.require_file_walk(&walk.generation),
            Self::Project => permit.require_project_walk(),
        }
    }
    pub(super) fn submit<'facts>(
        &self,
        prepared: &PreparedProgram,
        ready: &crate::application::file_dag::program::runtime::types::ReadySelection<'_, '_>,
        inputs: &[super::admission::AdmittedInput],
        owner: &std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
        read: &dyn Fn(Relation) -> Result<Rows<'facts>, OperatingError>,
    ) -> Result<bool, OperatingError> {
        self.validate(ready.permit())?;
        let Self::File(walk) = self else {
            return Ok(false);
        };
        let permit = ready.permit();
        if prepared.walk_class(permit.descriptor()).is_none() {
            return Ok(false);
        }
        let [input] = inputs else {
            return Err(OperatingError::WrongOperation);
        };
        let rows = input.rows(owner, read)?;
        let relation = walk.relation(prepared, permit)?;
        walk.submit(prepared, ready, relation, &rows)
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
        let Self::File(walk) = self else {
            return Ok(None);
        };
        let Some(class) = prepared.walk_class(permit.descriptor()) else {
            return Ok(None);
        };
        match (delivery, rows) {
            (WalkDelivery::Positions, Rows::Tree(_))
            | (WalkDelivery::Stored, Rows::Stored(_))
            | (WalkDelivery::Rows, _) => {}
            _ => return Ok(None),
        }
        let relation = walk.relation(prepared, permit)?;
        let Some(batch) = walk.prepare(prepared, permit, relation, rows, context, kernels)? else {
            return Ok(None);
        };
        // Result leases contain no RefCell guard or provider borrow. Reentrant
        // delivery may safely advance another admitted source in this file.
        Ok(batch
            .lease(class)
            .map(|(result, visits)| WalkResult { result, visits }))
    }
}
/// Physical delivery must decline before opening any shared predicate work.
#[derive(Clone, Copy)]
pub(super) enum WalkDelivery {
    Positions,
    Stored,
    Rows,
}
pub(super) struct WalkResult {
    result: std::sync::Arc<crate::domain::shared_walk::types::ClassResult>,
    visits: crate::domain::shared_walk::types::RowsVisited,
}
impl WalkResult {
    pub(super) fn result(&self) -> &crate::domain::shared_walk::types::ClassResult {
        &self.result
    }
    pub(super) fn visits(&self) -> crate::domain::shared_walk::types::RowsVisited {
        self.visits
    }
}

pub(super) struct FileWalk {
    generation: super::admission::FileGeneration,
    // Position batches live until this file owner drops. Last-consumer retirement
    // and accounting are not implemented by this source cache.
    batches: RefCell<HashMap<WalkSourceIdentity, SourceWork>>,
    work: RefCell<WalkWork>,
}
impl FileWalk {
    pub(super) fn new(generation: super::admission::FileGeneration) -> Self {
        Self {
            generation,
            batches: RefCell::new(HashMap::new()),
            work: RefCell::new(WalkWork::default()),
        }
    }
    fn relation(
        &self,
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
    /// Called only when the existing runtime has completed the head's demand program.
    /// A rule being requested is never sufficient to queue its other classes.
    fn submit(
        &self,
        prepared: &PreparedProgram,
        ready: &crate::application::file_dag::program::runtime::types::ReadySelection<'_, '_>,
        relation: Relation,
        rows: &Rows<'_>,
    ) -> Result<bool, OperatingError> {
        let permit = ready.permit();
        let Some(class) = prepared.walk_class(permit.descriptor()) else {
            return Ok(false);
        };
        permit.validate_program(prepared.program().as_ref())?;
        let Some(admitted) = permit.admit_walk_source(relation, rows)? else {
            return Ok(false);
        };
        if matches!(rows, Rows::Tree(tree) if tree.file != self.generation.file()) {
            return Err(OperatingError::NotRead);
        }
        let key = WalkSourceIdentity {
            population: admitted.population(),
            relation,
        };
        tracing::trace!(target: super::constants::WALK_TARGET,
            event = super::constants::WALK_READY,
            class = class.0, relation = ?relation, population = ?key.population);
        let mut sources = self.batches.borrow_mut();
        let source = sources.entry(key).or_insert_with(|| SourceWork {
            ready: BTreeSet::new(),
            batch: NativeBatch::for_program(prepared.walk()),
        });
        if source.batch.class(class).is_none() {
            source.ready.insert(class);
        }
        Ok(true)
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
            let request = PermittedWalk {
                permit,
                prepared,
                source: admitted,
                context,
                ready: &ready,
                work: &mut work,
            };
            let batch = match kernels.execute(super::super::executor::KernelCall::Walk { request })
            {
                Ok(super::super::executor::KernelAnswer::Walk(batch)) => batch,
                Err(super::super::executor::KernelFault::Refused(error)) => return Err(error),
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
            for class in &ready {
                source.ready.remove(class);
            }
        }
        Ref::filter_map(self.batches.borrow(), |batches| {
            batches.get(&identity).map(|source| &source.batch)
        })
        .map(Some)
        .map_err(|_| OperatingError::NotRun)
    }
    pub(super) fn counts(&self) -> crate::domain::shared_walk::types::WalkCounts {
        self.batches.borrow().values().fold(
            crate::domain::shared_walk::types::WalkCounts::default(),
            |mut total, batch| {
                let counts = batch.batch.counts();
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
            },
        )
    }
}
/// Only the live engine permit can bind these borrowed, already-admitted rows.
pub(crate) struct PermittedWalk<'call, 'context, 'scope> {
    permit: &'call KernelPermit<'scope>,
    prepared: &'call PreparedProgram,
    source: super::admission::AdmittedWalkSource<'call>,
    context: &'call mut OperatorContext<'context>,
    ready: &'call [ClassPlace],
    work: &'call mut WalkWork,
}
impl PermittedWalk<'_, '_, '_> {
    pub(in crate::application::file_dag) fn dispatch(self) -> Result<NativeBatch, OperatingError> {
        self.permit.validate_program(self.prepared.program())?;
        self.permit.validate_walk_source(&self.source)?;
        let batch = crate::domain::shared_walk::service::evaluate(
            self.prepared.walk(),
            self.source.relation(),
            self.source.rows(),
            self.prepared.all_bindings(),
            self.context,
            self.ready,
            self.work,
        );
        let counts = batch.counts();
        tracing::trace!(target: super::constants::WALK_TARGET, event = super::constants::WALK_DISPATCH,
            relation = ?self.source.relation(), population = ?self.source.population(),
            classes = ?self.ready.iter().filter(|class| batch.class(**class).is_some()).map(|class| class.0).collect::<Vec<_>>(),
            passes = counts.passes.0, source_rows = counts.source_rows.0,
            decoded_rows = counts.decoded_rows.0, predicate_rows = counts.predicate_rows.0,
            predicate_tests = counts.predicate_tests.0, predicate_reuses = counts.predicate_reuses.0,
            result_slots = counts.result_slots.0);
        Ok(batch)
    }
}
