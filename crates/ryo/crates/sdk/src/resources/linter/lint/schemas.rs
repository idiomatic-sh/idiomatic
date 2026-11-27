//! The shape of the data this entry does not control: a rule id and a glob checked from text, and the workers a caller asked for.

use ryo_execution::outbound::matching::interface::MatchingPort as _;
use ryo_matching::inbound::use_cases::matching::errors::MatchingError;
use ryo_values::domain::files::types::Glob as CoreGlob;
use ryo_values::domain::runs::types::WorkersAsked;

use super::types::{Glob, RuleId};

/// Why a text is not a rule id.
#[expect(
    missing_copy_implementations,
    reason = "adding `Copy` breaks the SDK against its semver baseline (cargo-semver-checks `copy_impl_added`): it comes with the SDK's next version"
)]
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("a rule id is not blank")]
pub struct RuleIdSchemaError;

/// Why a text is not a glob.
#[derive(Clone, Debug, PartialEq, Eq, thiserror::Error)]
#[error("`{glob}` is not a glob: {reason}")]
pub struct GlobSchemaError {
    /// The text.
    pub glob: String,
    /// Why.
    pub reason: String,
}

impl TryFrom<&str> for RuleId {
    type Error = RuleIdSchemaError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        if text.trim().is_empty() {
            return Err(RuleIdSchemaError);
        }
        Ok(Self(text.into()))
    }
}

impl TryFrom<&str> for Glob {
    type Error = GlobSchemaError;

    fn try_from(text: &str) -> Result<Self, Self::Error> {
        // A glob is the text the matching reads: the schema keeps the text, and the run makes it ready among its rule's globs.
        match super::composition::build_matching().parse(&CoreGlob(text.into())) {
            Ok(()) => Ok(Self(text.into())),
            Err(MatchingError::BadGlob { glob, reason }) => Err(GlobSchemaError { glob, reason }),
            Err(MatchingError::InvalidRegex { reason }) => Err(GlobSchemaError {
                glob: text.into(),
                reason,
            }),
        }
    }
}

/// The workers the caller asked for, as the scheduler factory takes them.
pub(super) const fn workers(workers: super::types::Workers) -> WorkersAsked {
    WorkersAsked(workers.0.get())
}
