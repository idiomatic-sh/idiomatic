//! The shape every item of the `comparisons` collection implements.

use crate::domain::comparing::errors::ComparingError;
use crate::domain::contract::types::PredicateOutcome;
use crate::domain::table_texts::types::Datum;
use ryo_values::domain::plan_parts::types::Test;

/// What every comparison family offers: whether one of its tests holds between two sides.
pub trait ComparisonsItem {
    /// Whether the test holds between `left` and `right`.
    ///
    /// # Errors
    ///
    /// `WrongTest` when the test is not one of this family's.
    fn holds(
        &self,
        test: Test,
        left: &Datum<'_>,
        right: &Datum<'_>,
    ) -> Result<PredicateOutcome, ComparingError>;
}
