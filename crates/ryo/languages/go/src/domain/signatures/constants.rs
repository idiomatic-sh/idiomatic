//! The words of the `signatures` concept: what a parameter is in Go's own words.

use super::types::SignatureRefinement;

/// The receiver of a method.
pub(crate) const RECEIVER: SignatureRefinement = SignatureRefinement("receiver");
/// A parameter.
pub(crate) const PARAMETER: SignatureRefinement = SignatureRefinement("parameter");
/// The parameter that takes the arguments left over, `...T`.
pub(crate) const VARIADIC: SignatureRefinement = SignatureRefinement("variadic");
/// A named result.
pub(crate) const RESULT: SignatureRefinement = SignatureRefinement("result");
