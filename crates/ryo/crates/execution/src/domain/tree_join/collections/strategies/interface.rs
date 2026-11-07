//! The shape every item of the `strategies` collection implements.

use crate::domain::tree_join::collections::strategies::types::Fitting;
use ryo_derived::domain::native::types::NativeFinding as Finding;

/// What every item of the `strategies` collection offers: whether it finds the right rows of a join that finds them by `finding`
/// (`None`: by the hash of a key, in a table).
pub(crate) trait StrategiesItem {
    /// Whether the strategy finds the rows for this finding.
    fn applies(&self, finding: Option<Finding>) -> Fitting;
}
