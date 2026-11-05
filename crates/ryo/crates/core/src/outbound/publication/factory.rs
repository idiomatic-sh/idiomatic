//! Construct the injected execution publication adapter at composition.
use super::{
    adapters::execution::adapter::PublicationExecutionAdapter, interface::PublicationPort,
};
use ryo_execution::inbound::use_cases::publication::interface::PublicationUseCase;
use std::sync::Arc;

#[must_use]
pub fn execution(publication: Arc<dyn PublicationUseCase>) -> impl PublicationPort {
    PublicationExecutionAdapter::new(publication)
}
