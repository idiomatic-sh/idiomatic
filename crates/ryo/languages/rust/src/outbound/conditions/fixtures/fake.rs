//! The fake of the `conditions` port: what the language's tests stand in for it with.

use super::super::interface::ConditionsPort;
use crate::domain::conditions::types::PlatformCondition;
use crate::domain::targets::types::Platform;

/// What the language's tests stand in for `conditions` with: a platform with its white space taken out, a space after each comma and
/// around each `=`. It knows nothing of quoted text.
#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct ConditionsFake;

impl ConditionsPort for ConditionsFake {
    fn show(&self, platform: &Platform) -> PlatformCondition {
        let compact: String = platform
            .0
            .chars()
            .filter(|each| !each.is_whitespace())
            .collect();
        PlatformCondition(compact.replace(',', ", ").replace('=', " = ").into())
    }
}

#[cfg(test)]
#[path = "fake_test.rs"]
mod tests;
