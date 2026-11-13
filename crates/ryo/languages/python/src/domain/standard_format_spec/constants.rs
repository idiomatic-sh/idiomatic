//! Target milestone owned by the standard formatting grammar.
use super::types::{StandardFormatTarget, TargetVersionPart};
pub(crate) const ZERO_COERCION_TARGET: StandardFormatTarget = StandardFormatTarget {
    major: TargetVersionPart(3),
    minor: TargetVersionPart(11),
};
