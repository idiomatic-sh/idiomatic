//! One admitted source population is scanned only after ordinary input admission.
use super::{admission::KernelPermit, facade::PreparedProgram};
use crate::{
    application::file_dag::derived::owner::PopulationId,
    domain::{
        executable_program::types::Instruction,
        operating::{errors::OperatingError, types::OperatorContext},
        positions::types::ClassPlace,
        shared_walk::types::{WalkWork, prepared_walk::value::NativeBatch},
        table_model::types::Rows,
    },
};
use ryo_relations::domain::relations::types::{FileId, Relation};
use std::{
    cell::{Ref, RefCell},
    collections::HashMap,
};
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
struct WalkSourceIdentity {
    population: PopulationId,
    relation: Relation,
}

struct SourceWork {
    ready: Vec<ClassPlace>,
    batch: NativeBatch,
}

pub(super) struct FileWalk {
    file: FileId,
    // Position batches live until this file owner drops. Last-consumer retirement
    // and accounting are not implemented by this source cache.
    batches: RefCell<HashMap<WalkSourceIdentity, SourceWork>>,
    work: RefCell<WalkWork>,
}
impl FileWalk {
    pub(super) fn new(file: FileId) -> Self {
        Self {
            file,
            batches: RefCell::new(HashMap::new()),
            work: RefCell::new(WalkWork::default()),
        }
    }
    pub(super) fn class(
        &self,
        prepared: &PreparedProgram,
        instruction: &Instruction,
    ) -> Option<ClassPlace> {
        prepared.walk_class(instruction)
    }
    /// Called only when the existing runtime has completed the head's demand program.
    /// A rule being requested is never sufficient to queue its other classes.
    pub(super) fn submit(
        &self,
        prepared: &PreparedProgram,
        permit: &KernelPermit<'_>,
        relation: Relation,
        rows: &Rows<'_>,
    ) -> Result<bool, OperatingError> {
        let Some(class) = self.class(prepared, permit.descriptor()) else {
            return Ok(false);
        };
        permit.validate_program(prepared.program().as_ref())?;
        let Some(admitted) = permit.admit_walk_source(relation, rows)? else {
            return Ok(false);
        };
        if matches!(rows, Rows::Tree(tree) if tree.file != self.file) {
            return Err(OperatingError::NotRead);
        }
        let key = WalkSourceIdentity {
            population: admitted.population(),
            relation,
        };
        let mut sources = self.batches.borrow_mut();
        let source = sources.entry(key).or_insert_with(|| SourceWork {
            ready: Vec::new(),
            batch: NativeBatch::for_program(prepared.walk()),
        });
        if source.batch.class(class).is_none() && !source.ready.contains(&class) {
            source.ready.push(class);
        }
        Ok(true)
    }

    pub(super) fn prepare<'walk>(
        &'walk self,
        prepared: &PreparedProgram,
        permit: &KernelPermit<'_>,
        relation: Relation,
        rows: &Rows<'_>,
        context: &mut OperatorContext<'_>,
        kernels: &dyn super::super::executor::PrimitiveExecutor,
    ) -> Result<Ref<'walk, NativeBatch>, OperatingError> {
        if let Rows::Tree(tree) = rows {
            if tree.file != self.file {
                return Err(OperatingError::NotRead);
            }
        }
        // Revalidate even a cache hit: population alone does not establish the
        // relation, current physical view, generation, or live invocation.
        permit.validate_program(prepared.program().as_ref())?;
        let admitted = permit
            .admit_walk_source(relation, rows)?
            .ok_or(OperatingError::NotRead)?;
        let identity = WalkSourceIdentity {
            population: admitted.population(),
            relation: admitted.relation(),
        };
        let ready = {
            let mut sources = self.batches.borrow_mut();
            let source = sources.get_mut(&identity).ok_or(OperatingError::NotRun)?;
            std::mem::take(&mut source.ready)
        };
        if !ready.is_empty() {
            let mut work = self.work.borrow_mut();
            let request = permit.walk(prepared, admitted, context, &ready, &mut work)?;
            let batch = match kernels.execute(super::super::executor::KernelCall::Walk { request })
            {
                Ok(super::super::executor::KernelAnswer::Walk(batch)) => batch,
                Err(super::super::executor::KernelFault::Refused(error)) => return Err(error),
                Err(super::super::executor::KernelFault::Portable(_)) => {
                    return Err(OperatingError::NotRead);
                }
                _ => return Err(OperatingError::WrongOperation),
            };
            self.batches
                .borrow_mut()
                .get_mut(&identity)
                .ok_or(OperatingError::NotRun)?
                .batch
                .merge(batch)?;
        }
        Ref::filter_map(self.batches.borrow(), |batches| {
            batches.get(&identity).map(|source| &source.batch)
        })
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
                total
            },
        )
    }
}
/// Only the live engine permit can bind these borrowed, already-admitted rows.
pub(crate) struct PermittedWalk<'call, 'context, 'scope> {
    pub(super) permit: &'call KernelPermit<'scope>,
    pub(super) prepared: &'call PreparedProgram,
    pub(super) source: super::admission::AdmittedWalkSource<'call>,
    pub(super) context: &'call mut OperatorContext<'context>,
    pub(super) ready: &'call [ClassPlace],
    pub(super) work: &'call mut WalkWork,
}
impl PermittedWalk<'_, '_, '_> {
    pub(in crate::application::file_dag) fn dispatch(self) -> Result<NativeBatch, OperatingError> {
        self.permit.validate_live()?;
        Ok(crate::domain::shared_walk::service::evaluate(
            self.prepared.walk(),
            self.source.relation(),
            self.source.rows(),
            self.prepared.all_bindings(),
            self.context,
            self.ready,
            self.work,
        ))
    }
}
