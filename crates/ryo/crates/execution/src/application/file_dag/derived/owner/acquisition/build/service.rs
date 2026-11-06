//! Builds one admitted derived storage through the semantic port.
use crate::application::file_dag::derived::owner::*;
use crate::application::file_dag::derived::owner::acquisition::admission::service::AdmittedViewInputs;

impl DerivedOwner {
    pub(in crate::application::file_dag::derived::owner::acquisition) fn build_admitted(
        &self,
        plan: &Arc<DerivedContract>,
        admitted: &AdmittedViewInputs<'_>,
        inputs: DerivedInputs<'_, '_>,
        scratch: &mut DerivedScratch,
        recycled: Option<DerivedStorage>,
    ) -> Result<Option<DerivedStorage>, OperatingError> {
        admitted.validate(self, plan, &inputs)?;
        crate::application::file_dag::derived::owner::acquisition::semantics::service::build(
            Arc::clone(&self.semantic),
            plan,
            inputs,
            scratch,
            recycled,
        )
    }
}
