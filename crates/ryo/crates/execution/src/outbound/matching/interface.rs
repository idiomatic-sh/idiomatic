//! What the `matching` port asks of the outside: globs made ready once, to be matched against many paths.

use super::errors::MatchingError;
use ryo_values::domain::files::types::Glob;
use ryo_values::domain::files::types::GlobsMatcher;

/// What every `matching` adapter implements.
pub trait MatchingPort: Send + Sync {
    /// Makes the globs ready to match a path against all of them at once. A `*` crosses no `/`; `**` crosses any.
    ///
    /// # Errors
    ///
    /// `BadGlob` for the first glob that is not one.
    fn compile(&self, globs: &[Glob]) -> Result<GlobsMatcher, MatchingError>;

    /// Whether `glob` is one, read and made ready for nothing: what a caller checks a glob with before a run.
    ///
    /// # Errors
    ///
    /// `BadGlob` when it is not one.
    fn parse(&self, glob: &Glob) -> Result<(), MatchingError>;
}
