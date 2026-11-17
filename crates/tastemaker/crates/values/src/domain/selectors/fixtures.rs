//! The test support of the `selectors` concept: compilers that compile nothing, for the cases that compile no pattern
//! or that ask what a refusal does.

use std::sync::Arc;

use crate::domain::selectors::types::{PathSet, PathTest, PatternFailure, SetGlob, Spelling};

/// A pattern compiler that refuses every spelling.
///
/// # Errors
///
/// Always: it compiles nothing.
pub fn refusing(_spelling: &Spelling) -> Result<Arc<dyn PathTest>, PatternFailure> {
    Err(PatternFailure("refused".into()))
}

/// A set compiler that refuses every set of globs.
///
/// # Errors
///
/// Always: it compiles nothing.
pub fn refusing_sets(_globs: &[SetGlob]) -> Result<Arc<dyn PathSet>, PatternFailure> {
    Err(PatternFailure("refused".into()))
}
