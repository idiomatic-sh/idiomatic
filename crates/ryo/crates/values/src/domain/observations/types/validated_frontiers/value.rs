//! The `ValidatedFrontiers` value: a sealed borrow of frontier readiness that only the observations service lends.

/// Only the common pure validation boundary can lend this checked view.
///
/// # Examples
///
/// ```
/// use ryo_values::domain::observations::service::{new_frontiers, validate_frontiers};
/// use ryo_values::domain::records::types::{ObservationReadinessCount, ObservationReadinessLayout};
/// use ryo_values::domain::stored_facts::types::Digest;
///
/// let layout = ObservationReadinessLayout(Digest([0; 16]));
/// let slots = ObservationReadinessCount(2);
/// let map = new_frontiers(layout, slots);
/// assert!(validate_frontiers(&map, layout, slots).is_ok());
/// ```
#[derive(Clone, Copy, Debug)]
pub struct ValidatedFrontiers<'a>(
    pub(in crate::domain::observations) &'a crate::domain::records::types::FrontierReadiness,
);
