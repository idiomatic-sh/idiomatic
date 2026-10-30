//! What the `conditions` port asks of the outside: how Cargo shows the platform a dependency is for.

use crate::domain::conditions::types::PlatformCondition;
use crate::domain::targets::types::Platform;

/// What every `conditions` adapter implements.
pub(crate) trait ConditionsPort: Send + Sync {
    /// How Cargo shows `platform`: a triple as it is, a `cfg(...)` expression normalized with one space after a comma and around an
    /// `=`; the platform as written when it is not an expression.
    fn show(&self, platform: &Platform) -> PlatformCondition;
}
