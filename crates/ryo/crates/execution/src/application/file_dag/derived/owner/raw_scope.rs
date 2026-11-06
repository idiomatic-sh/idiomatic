//! Explicitly unshared finite raw work uses the same acquisition and backing ledger.
use super::DerivedOwner;
use crate::application::file_dag::relations::RelationOwner;
use crate::domain::{
    derived_views::types::{DerivedContract, DerivedInputs, DerivedScratch, ViewRequest},
    operating::errors::OperatingError,
};
use ryo_derived::domain::contract::types::{DerivedView, RawOrder};
use std::{rc::Rc, sync::Arc};
pub(in crate::application::file_dag) struct RawScope {
    owner: Rc<RelationOwner>,
    prepared: Option<Arc<DerivedContract>>,
    admitted: Option<super::acquisition::admission::service::AdmittedViewInputs<'static>>,
    room: DerivedScratch,
    working: Option<
        crate::application::file_dag::derived::owner::acquisition::admission::service::AcquiredView,
    >,
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
        admitted: Option<super::acquisition::admission::service::AdmittedViewInputs<'static>>,
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
impl RawScope {
    /// The engine holds the required payload lease until the callback returns, including error
    /// paths. A borrowed order cannot escape the callback or retain upstream input authority.
    pub(in crate::application::file_dag) fn with_raw_order(
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

impl DerivedOwner {
    /// A finite semantic workspace belongs to this exact live population owner.
    /// The descriptor was prepared once by the request; undeclared raw orders refuse.
    pub(in crate::application::file_dag) fn finite_raw(
        &self,
        owner: std::rc::Rc<crate::application::file_dag::relations::RelationOwner>,
        prepared: Option<Arc<DerivedContract>>,
        permit: &crate::application::file_dag::program::runtime::service::KernelPermit<'_>,
    ) -> Result<RawScope, OperatingError> {
        if !std::ptr::eq(self, &owner.derived)
            || prepared
                .as_ref()
                .is_some_and(|plan| !matches!(&plan.request, ViewRequest::RawKeyOrder { .. }))
        {
            return Err(OperatingError::WrongOperation);
        }
        permit.validate_derived_owner(&owner)?;
        let admitted = prepared
            .as_ref()
            .map(|plan| self.admit_raw(plan, &owner, permit))
            .transpose()?;
        Ok(RawScope::for_program(owner, prepared, admitted))
    }
}
