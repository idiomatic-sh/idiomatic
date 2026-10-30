//! The conditions port, implemented with `cargo-platform`.

use std::str::FromStr as _;

use crate::domain::conditions::types::PlatformCondition;
use crate::domain::targets::types::Platform;
use crate::outbound::conditions::interface::ConditionsPort;

/// The `cargo-platform` conditions adapter.
#[derive(Debug)]
pub(crate) struct ConditionsCargoPlatformAdapter;

impl ConditionsPort for ConditionsCargoPlatformAdapter {
    fn show(&self, platform: &Platform) -> PlatformCondition {
        let written = platform.0.trim();
        PlatformCondition(
            cargo_platform::Platform::from_str(written)
                .map_or_else(|_| written.into(), |parsed| parsed.to_string().into()),
        )
    }
}
