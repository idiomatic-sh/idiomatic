//! One file-generation native scan, requested only after ordinary input admission.
use super::{admission::KernelPermit, facade::PreparedProgram};
use crate::domain::{
    executable_program::types::Instruction,
    operating::{errors::OperatingError, types::OperatorContext},
    positions::types::ClassPlace,
    shared_walk::types::{RuleActivity, WalkWork, prepared_walk::value::NativeBatch},
    table_cells::types::TreeRows,
};
use std::cell::RefCell;

pub(super) struct FileWalk {
    file: ryo_relations::domain::relations::types::FileId,
    active: Box<[RuleActivity]>,
    population: std::cell::Cell<Option<super::super::derived::owner::PopulationId>>,
    batch: RefCell<Option<NativeBatch>>,
    work: RefCell<WalkWork>,
}
impl FileWalk {
    pub(super) fn new(
        file: ryo_relations::domain::relations::types::FileId,
        demanded: &[bool],
    ) -> Self {
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
            population: std::cell::Cell::new(None),
            batch: RefCell::new(None),
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
    pub(super) fn prepare(
        &self,
        prepared: &PreparedProgram,
        permit: &KernelPermit<'_>,
        tree: TreeRows<'_>,
        context: &mut OperatorContext<'_>,
        kernels: &dyn super::super::executor::PrimitiveExecutor,
    ) -> Result<(), OperatingError> {
        if tree.file != self.file
            || tree.form != crate::domain::table_cells::types::TreeForm::Elements
        {
            return Err(OperatingError::NotRead);
        }
        let admitted = permit.admit_tree(tree)?;
        if self
            .population
            .get()
            .is_some_and(|expected| expected != admitted.population())
        {
            return Err(OperatingError::NotRead);
        }
        self.population.set(Some(admitted.population()));
        let mut batch = self.batch.borrow_mut();
        if batch.is_none() {
            let mut work = self.work.borrow_mut();
            let request = permit.walk(prepared, admitted, context, &self.active, &mut work)?;
            *batch = Some(
                match kernels.execute(super::super::executor::KernelCall::Walk { request }) {
                    Ok(super::super::executor::KernelAnswer::Walk(batch)) => batch,
                    Err(super::super::executor::KernelFault::Refused(error)) => return Err(error),
                    Err(super::super::executor::KernelFault::Portable(_)) => {
                        return Err(OperatingError::NotRead);
                    }
                    _ => return Err(OperatingError::WrongOperation),
                },
            );
        }
        Ok(())
    }
    pub(super) fn batch(&self) -> std::cell::Ref<'_, Option<NativeBatch>> {
        self.batch.borrow()
    }
}
/// The semantic adapter can consume this request, but cannot construct one.
pub(crate) struct PermittedWalk<'call, 'context, 'scope> {
    pub(super) permit: &'call KernelPermit<'scope>,
    pub(super) prepared: &'call PreparedProgram,
    pub(super) source: super::admission::AdmittedTree<'call>,
    pub(super) context: &'call mut OperatorContext<'context>,
    pub(super) active: &'call [RuleActivity],
    pub(super) work: &'call mut WalkWork,
}
impl PermittedWalk<'_, '_, '_> {
    pub(in crate::application::file_dag) fn dispatch(self) -> Result<NativeBatch, OperatingError> {
        self.permit.validate_live()?;
        Ok(crate::domain::shared_walk::service::evaluate(
            self.prepared.walk(),
            self.source.rows(),
            self.prepared.all_bindings(),
            self.context,
            self.active,
            self.work,
        ))
    }
}
