//! What the `matching` port asks of the outside, in the core's own vocabulary: a spelled pattern compiled into the test
//! the selectors read paths with.

use std::sync::Arc;

use crate::domain::selectors::types::{PathSet, PathTest, PatternFailure, Spelling};

/// What every `matching` adapter implements.
pub trait MatchingPort: Send + Sync {
    /// `spelling` compiled: its glob, with `/` a literal separator, and its expression when it has one.
    ///
    /// # Errors
    ///
    /// A failure naming why the glob or the expression does not compile.
    fn compile(&self, spelling: &Spelling) -> Result<Arc<dyn PathTest>, PatternFailure>;

    /// `globs` compiled together, each with `/` a literal separator and no expression: one pass tells which a path
    /// passes.
    ///
    /// # Errors
    ///
    /// A failure naming why a glob does not compile.
    fn compile_set(
        &self,
        globs: &[crate::domain::selectors::types::SetGlob],
    ) -> Result<Arc<dyn PathSet>, PatternFailure>;
}
