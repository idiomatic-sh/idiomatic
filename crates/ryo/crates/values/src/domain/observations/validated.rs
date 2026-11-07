//! Opaque semantic values. No scheduling, preparation or cache authority is carried here.

/// Only the common pure validation boundary can lend this checked view.
#[derive(Clone, Copy, Debug)]
pub struct ValidatedFrontiers<'a>(pub(super) &'a crate::domain::records::types::FrontierReadiness);
