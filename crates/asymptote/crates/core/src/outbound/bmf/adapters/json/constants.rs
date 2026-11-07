//! The words Bencher Metric Format uses, and what is said of a text it does not hold.

/// Why no output is no measurement.
pub(super) const NOTHING: &str = crate::domain::judgement::constants::NOTHING;
/// Why a number that is negative or not finite is no measurement.
pub(super) const NEGATIVE: &str = "a value and its bounds are numbers of zero or more";
/// Why a spread that does not hold its value is no measurement.
pub(super) const OUTSIDE: &str = "lower_value <= value <= upper_value";
/// Why an object with no measure is no measurement.
pub(super) const NO_MEASUREMENT: &str = "it holds no measurement";
/// What joins a benchmark and a measure.
pub(super) const SLASH: &str = "/";
/// What joins a name and what is said of it.
pub(super) const COLON: &str = ": ";
/// Why a benchmark or a measure with no name is no measurement.
pub(super) const NO_NAME: &str = "a benchmark and a measure are named";
