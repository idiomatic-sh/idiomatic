//! The words of the `signatures` concept: what a parameter is in Go's own words.

use super::types::Refinement;

/// The receiver of a method.
pub(crate) const RECEIVER: Refinement = Refinement("receiver");
/// A parameter.
pub(crate) const PARAMETER: Refinement = Refinement("parameter");
/// The parameter that takes the arguments left over, `...T`.
pub(crate) const VARIADIC: Refinement = Refinement("variadic");
/// A named result.
pub(crate) const RESULT: Refinement = Refinement("result");
