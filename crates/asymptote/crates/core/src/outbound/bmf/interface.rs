//! What the `bmf` port asks of the outside, in the core's own vocabulary.

use super::errors::BmfError;
use super::types::{BmfText, Measurements};

/// What every `bmf` adapter implements.
pub trait BmfPort: std::fmt::Debug + Send + Sync {
    /// The measurements of `text`, sorted by benchmark and measure: each `{ "<benchmark>": { "<measure>": { "value":
    /// n, "lower_value": n, "upper_value": n } } }` entry, the two bounds optional.
    ///
    /// # Errors
    ///
    /// `Unparsable` when it is not an object of objects of metrics, a metric has no numeric `value` or a bound that is
    /// not numeric, or it holds no measurement at all.
    fn read(&self, text: &BmfText) -> Result<Measurements, BmfError>;
}
