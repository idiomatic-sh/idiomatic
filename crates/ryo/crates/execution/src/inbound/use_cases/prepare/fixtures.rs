//! The test support of the `prepare` use case: the derived views over a derived port the tests inject, where
//! production composes them once at the edge ([`crate::derived`]).
use super::types::DerivedViews;
use crate::application::derived::service::DerivedPreparation;
use ryo_execution_store::outbound::derived::interface::DerivedPort;
use std::sync::Arc;

/// The views over a derived port.
impl From<Arc<dyn DerivedPort>> for DerivedViews {
    fn from(port: Arc<dyn DerivedPort>) -> Self {
        Self(DerivedPreparation::from(port))
    }
}

/// The views over a derived port.
impl<T: DerivedPort + 'static> From<Arc<T>> for DerivedViews {
    fn from(port: Arc<T>) -> Self {
        Self(DerivedPreparation::from(port))
    }
}
