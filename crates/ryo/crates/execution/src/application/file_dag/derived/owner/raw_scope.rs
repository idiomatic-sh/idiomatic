//! Explicitly unshared finite raw work uses the same acquisition and backing ledger.
use crate::application::file_dag::relations::RelationOwner;
use crate::domain::operator::contract::derived::ScopedDerived;
use crate::domain::{
    derived_views::types::{DerivedContract, DerivedInputs, DerivedScratch, ViewRequest},
    operating::errors::OperatingError,
};
use ryo_derived::domain::contract::types::{DerivedView, RawOrder};
use std::{rc::Rc, sync::Arc};
pub(in crate::application::file_dag) struct RawScope {
    owner: Rc<RelationOwner>,
    prepared: Option<Arc<DerivedContract>>,
    admitted: Option<super::acquisition::service::AdmittedViewInputs<'static>>,
    room: DerivedScratch,
    working:
        Option<crate::application::file_dag::derived::owner::acquisition::service::AcquiredView>,
}
impl std::fmt::Debug for RawScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawScope").finish_non_exhaustive()
    }
}
impl RawScope {
    pub(super) fn for_program(
        owner: Rc<RelationOwner>,
        prepared: Option<Arc<DerivedContract>>,
        admitted: Option<super::acquisition::service::AdmittedViewInputs<'static>>,
    ) -> Self {
        Self {
            owner,
            prepared,
            admitted,
            room: DerivedScratch::default(),
            working: None,
        }
    }
}
impl ScopedDerived for RawScope {
    fn with_raw_order(
        &mut self,
        keys: &[u64],
        stride: usize,
        body: &mut dyn FnMut(&RawOrder) -> Result<(), OperatingError>,
    ) -> Result<(), OperatingError> {
        let request = ViewRequest::RawKeyOrder { stride };
        let plan = self
            .prepared
            .as_ref()
            .filter(|plan| super::super::request::identity::same_request(&plan.request, &request))
            .cloned()
            .ok_or(OperatingError::WrongOperation)?;
        let acquired = self.owner.derived.scoped_build(
            crate::application::file_dag::route_counts::BuildRoute::OwnerAcquire,
            &plan,
            self.admitted
                .as_ref()
                .ok_or(OperatingError::WrongOperation)?,
            DerivedInputs::RawWords(keys),
            self.working.take(),
            &mut self.room,
        )?;
        let result = {
            let DerivedView::RawKeyOrder(order) = acquired.view()? else {
                return Err(OperatingError::WrongOperation);
            };
            body(order)
        };
        if result.is_ok() {
            self.working = Some(acquired);
        }
        result
    }
}
