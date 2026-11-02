//! Request-bound storage ownership; semantic consumers receive only the trait's borrowed view.
use crate::domain::{
    derived_views::{
        collections::views::registration::registered,
        types::{DerivedContract, DerivedStorage, DerivedView},
    },
    operating::errors::OperatingError,
};
use std::sync::Arc;
pub(crate) struct AcquiredView {
    pub(super) plan: Arc<DerivedContract>,
    pub(super) storage: Arc<DerivedStorage>,
}
impl AcquiredView {
    pub(in crate::application::file_dag) fn new(
        plan: &Arc<DerivedContract>,
        storage: Arc<DerivedStorage>,
    ) -> Self {
        Self {
            plan: Arc::clone(plan),
            storage,
        }
    }
    pub(in crate::application::file_dag) fn view(&self) -> Result<DerivedView<'_>, OperatingError> {
        registered(self.plan.kind).view(&self.plan, &self.storage)
    }
}
