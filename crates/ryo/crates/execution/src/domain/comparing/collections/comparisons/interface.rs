//! The shape every item of the `comparisons` collection implements.

use crate::domain::operating::errors::OperatingError;
use crate::domain::table_model::types::Verdict;
use crate::domain::table_texts::types::Datum;
use ryo_values::domain::plan_parts::types::Test;

/// What every comparison family offers: whether one of its tests holds between two sides.
pub(crate) trait ComparisonsItem {
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
    ) -> Result<Verdict, OperatingError>;
}
