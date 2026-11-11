//! The support of the `identical` cases.
use std::sync::Arc;

pub(super) fn engine() -> Arc<dyn crate::outbound::operators::interface::OperatorsPort> {
    Arc::new(crate::application::file_dag::fixtures::operators())
}
