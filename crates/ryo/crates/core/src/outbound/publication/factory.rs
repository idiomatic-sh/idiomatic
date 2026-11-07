//! Construct the injected execution publication adapter at composition.
use super::{
    adapters::execution::adapter::PublicationExecutionAdapter, interface::PublicationPort,
};

#[must_use]
pub fn execution() -> impl PublicationPort {
    PublicationExecutionAdapter::new()
}
