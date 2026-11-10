//! The test support of the `observations` concept: the frontier checks the tests of other crates make.

use super::errors::ObservationsError;
use super::service;
use super::validated::ValidatedFrontiers;
use crate::domain::records::types::{
    FrontierReadiness, ObservationReadinessCount, ObservationReadinessLayout,
    ObservationReadinessSlot, ProducerReadiness,
};

/// Checks the frontier readiness of a request against its layout.
///
/// # Errors
///
/// [`ObservationsError::Invalid`] when the map is not the request's.
pub fn validate_frontiers(
    map: &FrontierReadiness,
    layout: ObservationReadinessLayout,
    slots: ObservationReadinessCount,
) -> Result<ValidatedFrontiers<'_>, ObservationsError> {
    service::validate_frontiers(map, layout, slots)
}

/// The readiness of the producer of a slot.
///
/// # Errors
///
/// [`ObservationsError::Invalid`] when the slot is outside the map.
pub fn frontier_state(
    map: ValidatedFrontiers<'_>,
    slot: ObservationReadinessSlot,
) -> Result<ProducerReadiness, ObservationsError> {
    service::frontier_state(map, slot)
}
