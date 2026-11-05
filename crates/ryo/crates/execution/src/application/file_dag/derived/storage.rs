//! Engine-held opaque semantic payload; only this application layer retains backing handles.
use crate::domain::operating::errors::OperatingError;
use crate::domain::operator::contract::derived::portable_fault;
use crate::domain::stored_graph::types::GraphIdentity;
use crate::outbound::derived::interface::DerivedPort;
use ryo_derived::domain::contract::types::{DerivedPayload, DerivedView, PreparedDerived};
use std::sync::Arc;
#[derive(Debug)]
pub(in crate::application::file_dag) struct DerivedStorage {
    port: Arc<dyn DerivedPort>,
    plan: Arc<PreparedDerived>,
    payload: Arc<DerivedPayload>,
    graph_identity: Option<Arc<GraphIdentity>>,
}
impl DerivedStorage {
    pub(in crate::application::file_dag::derived) fn new(
        port: Arc<dyn DerivedPort>,
        plan: Arc<PreparedDerived>,
        payload: DerivedPayload,
        graph_identity: Option<Arc<GraphIdentity>>,
    ) -> Self {
        Self {
            port,
            plan,
            payload: Arc::new(payload),
            graph_identity,
        }
    }
    pub(in crate::application::file_dag) fn view(&self) -> Result<DerivedView<'_>, OperatingError> {
        self.port
            .view(&self.plan, &self.payload)
            .map_err(portable_fault)
    }
    pub(in crate::application::file_dag::derived) fn payload(&self) -> &DerivedPayload {
        &self.payload
    }
    pub(in crate::application::file_dag::derived) fn graph_identity(
        &self,
    ) -> Option<&Arc<GraphIdentity>> {
        self.graph_identity.as_ref()
    }
    /// Describe the exact physical allocations; admission and lease classes remain with the owner.
    pub(in crate::application::file_dag::derived) fn backings(
        this: &Arc<Self>,
    ) -> Result<Arc<[super::ledger::Backing]>, OperatingError> {
        use super::ledger::Backing;
        let facts = this.payload.footprint().map_err(portable_fault)?;
        let mut output: Vec<Backing> = facts
            .shared
            .into_iter()
            .map(|fact| Backing {
                key: (fact.tag, fact.allocation_key),
                bytes: fact.bytes,
                retention_units: fact.position_capacity,
            })
            .collect();
        let bytes = (size_of::<Self>() + 2 * size_of::<usize>())
            .checked_add(facts.owned_bytes)
            .ok_or(OperatingError::ShapeMismatch)?;
        output.push(Backing {
            key: (8, Arc::as_ptr(this) as usize),
            bytes,
            retention_units: 0,
        });
        if let Some(identity) = &this.graph_identity {
            let keys = identity
                .keys
                .capacity()
                .checked_mul(size_of::<ryo_values::domain::plan_parts::types::ColumnIndex>())
                .ok_or(OperatingError::ShapeMismatch)?;
            output.push(Backing {
                key: (34, Arc::as_ptr(identity) as usize),
                bytes: (size_of::<GraphIdentity>() + 2 * size_of::<usize>())
                    .checked_add(keys)
                    .ok_or(OperatingError::ShapeMismatch)?,
                retention_units: 0,
            });
            if let Some(rows) = &identity.population.positions {
                let capacity_bytes = rows
                    .capacity()
                    .checked_mul(size_of::<u32>())
                    .ok_or(OperatingError::ShapeMismatch)?;
                output.push(Backing {
                    key: (1, Arc::as_ptr(rows) as usize),
                    bytes: (size_of::<Vec<u32>>() + 2 * size_of::<usize>())
                        .checked_add(capacity_bytes)
                        .ok_or(OperatingError::ShapeMismatch)?,
                    retention_units: rows.capacity(),
                });
            }
        }
        Ok(output.into())
    }
    pub(in crate::application::file_dag::derived) fn into_recycled(self) -> Option<DerivedPayload> {
        match Arc::try_unwrap(self.payload) {
            Ok(payload) => Some(payload),
            Err(_shared) => None,
        }
    }
}
