//! The immutable storage one semantic build produced, with the facts its owner accounts.
use crate::application::file_dag::derived::ledger::types::{
    BackingAddress, BackingKey, BackingTag, ByteCount, UnitCount,
};
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
    pub(super) fn new(
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
    pub(in crate::application::file_dag::derived::owner) fn view(
        &self,
    ) -> Result<DerivedView<'_>, OperatingError> {
        self.port
            .view(&self.plan, &self.payload)
            .map_err(portable_fault)
    }
    pub(in crate::application::file_dag::derived::owner) fn payload(&self) -> &DerivedPayload {
        &self.payload
    }
    pub(in crate::application::file_dag::derived::owner) fn graph_identity(
        &self,
    ) -> Option<&Arc<GraphIdentity>> {
        self.graph_identity.as_ref()
    }
    /// Describe the exact physical allocations; admission and lease classes remain with the owner.
    pub(in crate::application::file_dag::derived::owner) fn backings(
        this: &Arc<Self>,
    ) -> Result<Arc<[crate::application::file_dag::derived::ledger::types::Backing]>, OperatingError>
    {
        use crate::application::file_dag::derived::ledger::types::Backing;
        let facts = this.payload.footprint().map_err(portable_fault)?;
        let mut output: Vec<Backing> = facts
            .shared
            .into_iter()
            .map(|fact| Backing {
                key: BackingKey {
                    tag: BackingTag(fact.tag),
                    address: BackingAddress(fact.allocation_key),
                },
                bytes: ByteCount(fact.bytes),
                retention_units: UnitCount(fact.position_capacity),
            })
            .collect();
        let bytes = (size_of::<Self>() + 2 * size_of::<usize>())
            .checked_add(facts.owned_bytes)
            .ok_or(OperatingError::ShapeMismatch)?;
        output.push(Backing {
            key: BackingKey {
                tag: BackingTag(8),
                address: BackingAddress(Arc::as_ptr(this) as usize),
            },
            bytes: ByteCount(bytes),
            retention_units: UnitCount(0),
        });
        if let Some(identity) = &this.graph_identity {
            let keys = crate::application::file_dag::derived::ledger::capacity::checked::<
                ryo_values::domain::plan_parts::types::ColumnIndex,
            >(
                crate::application::file_dag::derived::ledger::types::SlotCount(
                    identity.keys.capacity(),
                ),
            )
            .ok_or(OperatingError::ShapeMismatch)?;
            output.push(Backing {
                key: BackingKey {
                    tag: BackingTag(34),
                    address: BackingAddress(Arc::as_ptr(identity) as usize),
                },
                bytes: ByteCount(
                    (size_of::<GraphIdentity>() + 2 * size_of::<usize>())
                        .checked_add(keys.0)
                        .ok_or(OperatingError::ShapeMismatch)?,
                ),
                retention_units: UnitCount(0),
            });
            if let Some(rows) = &identity.population.positions {
                output.push(Backing {
                    key: BackingKey {
                        tag: BackingTag(1),
                        address: BackingAddress(Arc::as_ptr(rows) as usize),
                    },
                    bytes: crate::application::file_dag::derived::ledger::capacity::shared_vector_checked(rows)
                        .ok_or(OperatingError::ShapeMismatch)?,
                    retention_units: UnitCount(rows.capacity()),
                });
            }
        }
        Ok(output.into())
    }
    pub(super) fn into_recycled(self) -> Option<DerivedPayload> {
        match Arc::try_unwrap(self.payload) {
            Ok(payload) => Some(payload),
            Err(_shared) => None,
        }
    }
}
