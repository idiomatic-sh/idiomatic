//! The `storage` application operation.

use crate::application::file_dag::derived::ledger::types::{
    BackingAddress, BackingKey, BackingTag, ByteCount, UnitCount,
};
use crate::outbound::derived::interface::DerivedPort;
use ryo_derived::domain::contract::types::DerivedView;
use ryo_derived::domain::contract::types::derived_payload::value::DerivedPayload;
use ryo_derived::domain::contract::types::prepared_derived::value::PreparedDerived;
use ryo_execution_domain::domain::operating::errors::OperatingError;
use ryo_execution_domain::domain::operating::service::derived_fault;
use ryo_execution_domain::domain::stored_graph::types::GraphIdentity;
use std::sync::Arc;

/// The built payload of a view and the prepared request it was built from.
#[derive(Debug)]
pub struct DerivedStorage {
    port: Arc<dyn DerivedPort>,
    plan: Arc<PreparedDerived>,
    payload: Arc<DerivedPayload>,
    graph_identity: Option<Arc<GraphIdentity>>,
}

impl DerivedStorage {
    pub(crate) fn new(
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
    /// The view over the built payload.
    ///
    /// # Errors
    ///
    /// The derived port's refusal to view the payload, mapped into its [`OperatingError`].
    pub fn view(&self) -> Result<DerivedView<'_>, OperatingError> {
        self.port
            .view(&self.plan, &self.payload)
            .map_err(derived_fault)
    }
    /// The built payload.
    pub fn payload(&self) -> &DerivedPayload {
        &self.payload
    }
    pub(crate) fn graph_identity(&self) -> Option<&Arc<GraphIdentity>> {
        self.graph_identity.as_ref()
    }
    /// Describe the exact physical allocations; admission and lease classes remain with the owner.
    pub(crate) fn backings(
        this: &Arc<Self>,
    ) -> Result<Arc<[crate::application::file_dag::derived::ledger::types::Backing]>, OperatingError>
    {
        use crate::application::file_dag::derived::ledger::types::Backing;
        let facts = ryo_derived::domain::footprint::service::describe(&this.payload)
            .map_err(derived_fault)?;
        let mut output: Vec<Backing> = facts
            .shared
            .into_iter()
            .map(|fact| Backing {
                key: BackingKey {
                    tag: BackingTag(fact.tag.0),
                    address: BackingAddress(fact.allocation_key.0),
                },
                bytes: ByteCount(fact.bytes.0),
                retention_units: UnitCount(fact.position_capacity.0),
            })
            .collect();
        let bytes = (size_of::<Self>() + 2 * size_of::<usize>())
            .checked_add(facts.owned_bytes.0)
            .ok_or(OperatingError::ShapeMismatch)?;
        output.push(Backing {
            key: BackingKey {
                tag: BackingTag(8),
                address: BackingAddress(Arc::as_ptr(this).addr()),
            },
            bytes: ByteCount(bytes),
            retention_units: UnitCount(0),
        });
        if let Some(identity) = &this.graph_identity {
            let keys = crate::application::file_dag::derived::ledger::capacity::service::checked::<
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
                    address: BackingAddress(Arc::as_ptr(identity).addr()),
                },
                bytes: ByteCount(
                    (size_of::<GraphIdentity>() + 2 * size_of::<usize>())
                        .checked_add(keys.0)
                        .ok_or(OperatingError::ShapeMismatch)?,
                ),
                retention_units: UnitCount(0),
            });
        }
        Ok(output.into())
    }
    pub(crate) fn into_recycled(self) -> Option<DerivedPayload> {
        match Arc::try_unwrap(self.payload) {
            Ok(payload) => Some(payload),
            Err(_shared) => None,
        }
    }
}
