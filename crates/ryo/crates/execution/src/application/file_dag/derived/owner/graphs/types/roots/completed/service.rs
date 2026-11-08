//! Complete a root answer with a required lease over the graph and its reached rows.
use super::super::super::{
    handles::service::RootAnswer,
    retained::types::{GraphEntry, ReachedVertices},
};
use crate::application::file_dag::derived::{
    ledger::{
        capacity::{arc_bytes, vector},
        types::{Backing, BackingAddress, BackingKey, BackingTag, ByteCount, UnitCount},
    },
    owner::DerivedOwner,
};
use ryo_execution_domain::domain::operating::errors::OperatingError;
use std::sync::Arc;

pub(crate) fn completed(
    owner: &DerivedOwner,
    graph: &GraphEntry,
    rows: Arc<ReachedVertices>,
) -> Result<RootAnswer, OperatingError> {
    let mut backings = graph.backings.to_vec();
    backings.push(Backing {
        key: BackingKey {
            tag: BackingTag(20),
            address: BackingAddress(Arc::as_ptr(&rows) as usize),
        },
        bytes: ByteCount(arc_bytes::<Vec<u32>>().0 + vector(&rows.0).0),
        retention_units: UnitCount(0),
    });
    let lease = owner.required_lease(&backings)?;
    Ok(RootAnswer::new(rows, Arc::clone(&graph.storage), lease))
}
