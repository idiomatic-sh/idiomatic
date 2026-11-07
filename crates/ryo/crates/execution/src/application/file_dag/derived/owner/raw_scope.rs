//! Explicitly unshared finite raw work uses the same acquisition and backing ledger.
use super::DerivedOwner;
use crate::application::file_dag::relations::RelationOwner;
use crate::domain::{
    derived_views::types::{contract::value::DerivedContract, DerivedInputs, DerivedScratch, ViewRequest},
    operating::errors::OperatingError,
};
use ryo_derived::domain::contract::types::DerivedView;
use ryo_derived::domain::raw_order::types::raw_order::value::RawOrder;
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
        f.debug_struct(crate::application::file_dag::derived::constants::RAW_SCOPE_NAME)
            .finish_non_exhaustive()
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
        keys: &crate::domain::positions::types::Keys,
        stride: crate::domain::positions::types::Stride,
        body: &mut dyn FnMut(&RawOrder) -> Result<(), OperatingError>,
    ) -> Result<(), OperatingError> {
        let request = ViewRequest::RawKeyOrder { stride };
        let plan = self
            .prepared
            .as_ref()
            .filter(|plan| {
                super::super::request::identity::types::StaticRequest(&plan.request)
                    == super::super::request::identity::types::StaticRequest(&request)
            })
            .cloned()
            .ok_or(OperatingError::WrongOperation)?;
        let acquired = self.owner.derived().scoped_build(
            crate::application::file_dag::route_counts::types::BuildRoute::OwnerAcquire,
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
        if !std::ptr::eq(self, owner.derived())
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

impl RawScope {
    /// The owner orders the keys the kernel names through its raw scope and lends the kernel the
    /// admitted order for the call; the kernel never sees the scope or the acquisition path.
    pub(in crate::application::file_dag) fn evaluate_native(
        &mut self,
        semantics: &dyn crate::domain::operator::contract::types::PrimitiveSemantics,
        operation: &ryo_values::domain::plans::types::Operation,
        rows: &[crate::domain::table_model::types::Rows<'_>],
        shape: &crate::domain::lowering::types::LoweredShape,
        context: &mut crate::domain::operating::types::OperatorContext<'_>,
        work: &mut crate::domain::native_positions::types::PositionWork,
        counts: &crate::domain::operator::contract::types::WorkCounters,
    ) -> Result<Option<crate::domain::positions::types::Positions>, OperatingError> {
        let Some(staged) = crate::domain::native_positions::rows::stage(rows)? else {
            return Ok(None);
        };
        let Some(raw) = staged.raw_keys(operation, shape, work, counts)? else {
            return staged
                .evaluate(semantics, None, operation, shape, context, work, counts)
                .map(Some);
        };
        let mut positions = None;
        let ordered = self.with_raw_order(&raw.keys, raw.stride, &mut |order| {
            positions = Some(staged.evaluate(
                semantics,
                Some(order),
                operation,
                shape,
                context,
                work,
                counts,
            )?);
            Ok(())
        });
        work.keys = raw.keys;
        ordered?;
        Ok(positions)
    }
}
