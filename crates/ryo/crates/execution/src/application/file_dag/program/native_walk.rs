//! One admitted source population is scanned only after ordinary input admission.
use super::{admission::KernelPermit, facade::PreparedProgram};
use crate::{
    application::file_dag::derived::owner::PopulationId,
    domain::{
        executable_program::types::Instruction,
        operating::{errors::OperatingError, types::OperatorContext},
        positions::types::ClassPlace,
        shared_walk::types::{RuleActivity, WalkWork, prepared_walk::value::NativeBatch},
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

pub(super) struct FileWalk {
    file: FileId,
    active: Box<[RuleActivity]>,
    // Position batches live until this file owner drops. Last-consumer retirement
    // and accounting are not implemented by this source cache.
    batches: RefCell<HashMap<WalkSourceIdentity, NativeBatch>>,
    work: RefCell<WalkWork>,
}
impl FileWalk {
    pub(super) fn new(file: FileId, demanded: &[bool]) -> Self {
        Self {
            file,
            active: demanded
                .iter()
                .map(|active| {
                    if *active {
                        RuleActivity::Active
                    } else {
                        RuleActivity::Inactive
                    }
                })
                .collect(),
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
        let admitted = permit.admit_walk_source(relation, rows)?;
        let identity = WalkSourceIdentity {
            population: admitted.population(),
            relation: admitted.relation(),
        };
        if !self.batches.borrow().contains_key(&identity) {
            let mut work = self.work.borrow_mut();
            let request = permit.walk(prepared, admitted, context, &self.active, &mut work)?;
            let batch = match kernels.execute(super::super::executor::KernelCall::Walk { request })
            {
                Ok(super::super::executor::KernelAnswer::Walk(batch)) => batch,
                Err(super::super::executor::KernelFault::Refused(error)) => return Err(error),
                Err(super::super::executor::KernelFault::Portable(_)) => {
                    return Err(OperatingError::NotRead);
                }
                _ => return Err(OperatingError::WrongOperation),
            };
            let _previous = self.batches.borrow_mut().insert(identity, batch);
        }
        Ref::filter_map(self.batches.borrow(), |batches| batches.get(&identity))
            .map_err(|_| OperatingError::NotRun)
    }
    pub(super) fn counts(&self) -> crate::domain::shared_walk::types::WalkCounts {
        self.batches.borrow().values().fold(
            crate::domain::shared_walk::types::WalkCounts::default(),
            |mut total, batch| {
                let counts = batch.counts();
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
    pub(super) active: &'call [RuleActivity],
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
            self.active,
            self.work,
        ))
    }
}
