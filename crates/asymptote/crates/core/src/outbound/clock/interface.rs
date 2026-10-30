//! What the `clock` port asks of the outside, in the core's own vocabulary.

use crate::domain::operations::types::Day;

/// What every `clock` adapter implements.
pub trait ClockPort: std::fmt::Debug + Send + Sync {
    /// Today, `YYYY-MM-DD`.
    fn today(&self) -> Day;
}
