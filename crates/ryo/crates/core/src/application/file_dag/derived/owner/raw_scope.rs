//! Unproven raw key populations stay required-live and explicitly unshared.
use crate::application::file_dag::derived::ledger::Backing;
use crate::application::file_dag::relations::RelationOwner;
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::derived::{ScopedDerived, portable_fault};
use ryo_derived::domain::contract::types::{
    BuildOutcome, BuildWorkspace, DerivedInputs, DerivedPayload, DerivedView, PreparedDerived,
    RawOrder, ViewRequest,
};
use std::{rc::Rc, sync::Arc};
struct Working {
    // Release bookkeeping before dropping payload allocations/dependencies.
    lease: Arc<crate::application::file_dag::derived::ledger::ResourceLease>,
    payload: Arc<DerivedPayload>,
}
impl Working {
    fn recycle(self) -> Option<DerivedPayload> {
        let Self { lease, payload } = self;
        drop(lease);
        Arc::try_unwrap(payload).ok()
    }
}
pub(in crate::application::file_dag) struct RawScope {
    owner: Rc<RelationOwner>,
    prepared: Option<Arc<PreparedDerived>>,
    room: BuildWorkspace,
    working: Option<Working>,
    finite: bool,
}
impl std::fmt::Debug for RawScope {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawScope").finish_non_exhaustive()
    }
}
impl RawScope {
    pub(in crate::application::file_dag) fn new(owner: Rc<RelationOwner>) -> Self {
        Self {
            owner,
            prepared: None,
            room: BuildWorkspace::default(),
            working: None,
            finite: false,
        }
    }
    pub(in crate::application::file_dag) fn for_program(
        owner: Rc<RelationOwner>,
        prepared: Option<Arc<PreparedDerived>>,
    ) -> Self {
        Self {
            owner,
            prepared,
            room: BuildWorkspace::default(),
            working: None,
            finite: true,
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
        let port = self.owner.derived.semantic_port();
        let request = ViewRequest::RawKeyOrder { stride };
        let prepared = match &self.prepared {
            Some(prepared) if prepared.request() == &request => Arc::clone(prepared),
            _ if self.finite => return Err(OperatingError::WrongOperation),
            _ => {
                let prepared = port.prepare(request, Vec::new());
                self.prepared = Some(Arc::clone(&prepared));
                prepared
            }
        };
        let recycled = self.working.take().and_then(Working::recycle);
        let answer = port
            .build(
                &prepared,
                DerivedInputs::RawWords(keys),
                &mut self.room,
                recycled,
            )
            .map_err(portable_fault)?;
        let BuildOutcome::Built(payload) = answer else {
            return Err(OperatingError::WrongOperation);
        };
        let payload = Arc::new(payload);
        let footprint = payload.footprint().map_err(portable_fault)?;
        let mut backings = Vec::with_capacity(footprint.shared.len() + 1);
        backings.push(Backing {
            key: (17, Arc::as_ptr(&payload) as usize),
            bytes: footprint.owned_bytes,
            retention_units: 0,
        });
        backings.extend(footprint.shared.into_iter().map(|part| Backing {
            key: (part.tag, part.allocation_key),
            bytes: part.bytes,
            retention_units: part.position_capacity,
        }));
        let lease = self.owner.derived.required_lease(&backings)?;
        let view = port.view(&prepared, &payload).map_err(portable_fault)?;
        let DerivedView::RawKeyOrder(order) = view else {
            return Err(OperatingError::WrongOperation);
        };
        let result = body(order);
        if result.is_ok() {
            self.working = Some(Working { lease, payload });
        }
        // Error leaves no working handle or required lease; there is never an optional entry.
        result
    }
}
