//! The unknowns document of the postcard store: its wire shapes and their conversion from and into the core's tables.

use super::super::strings::schemas::{StringsSchema, TextSchema, named, text};
use ryo_language::domain::files::types::FilePath;
use ryo_values::domain::runs::types::{ApplicabilityRefusal, RuleId, UnknownCause};
use ryo_values::domain::stored_findings::types::{StoredUnknown, UnknownText};
use std::borrow::Cow;

/// Stable tags of why a rule could not decide, in the checked store.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) enum CauseSchema {
    Undecided,
    SyntaxUndetermined,
    MetadataUnavailable,
}

/// Something a rule could not decide: one row for each.
#[derive(Clone, Copy, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(in crate::outbound::lint_store::adapters::postcard) struct UnknownSchema {
    /// The rule.
    pub(in crate::outbound::lint_store::adapters::postcard) rule: TextSchema,
    /// The file whose relation is read in part.
    pub(in crate::outbound::lint_store::adapters::postcard) path: TextSchema,
    /// What is partial there and why.
    pub(in crate::outbound::lint_store::adapters::postcard) said: TextSchema,
    /// Why the rule could not decide.
    pub(in crate::outbound::lint_store::adapters::postcard) cause: CauseSchema,
}

/// The row of an unknown, its texts named in the table of strings.
pub(in crate::outbound::lint_store::adapters::postcard) fn unknown_schema<'a>(
    strings: &mut StringsSchema<'a>,
    unknown: &'a StoredUnknown,
) -> UnknownSchema {
    UnknownSchema {
        rule: text(strings, &unknown.rule.0),
        path: text(strings, &unknown.path.0),
        said: text(strings, &unknown.message.0),
        cause: match unknown.cause {
            UnknownCause::Undecided => CauseSchema::Undecided,
            UnknownCause::Refused(ApplicabilityRefusal::SyntaxUndetermined) => {
                CauseSchema::SyntaxUndetermined
            }
            UnknownCause::Refused(ApplicabilityRefusal::MetadataUnavailable) => {
                CauseSchema::MetadataUnavailable
            }
        },
    }
}

/// The unknown a row is of, its texts read from the table of strings.
pub(in crate::outbound::lint_store::adapters::postcard) fn unknown_from(
    strings: &[Cow<'_, str>],
    unknown: &UnknownSchema,
) -> Result<StoredUnknown, postcard::Error> {
    Ok(StoredUnknown {
        rule: RuleId(named(strings, unknown.rule)?),
        path: FilePath(named(strings, unknown.path)?),
        message: UnknownText(named(strings, unknown.said)?),
        cause: match unknown.cause {
            CauseSchema::Undecided => UnknownCause::Undecided,
            CauseSchema::SyntaxUndetermined => {
                UnknownCause::Refused(ApplicabilityRefusal::SyntaxUndetermined)
            }
            CauseSchema::MetadataUnavailable => {
                UnknownCause::Refused(ApplicabilityRefusal::MetadataUnavailable)
            }
        },
    })
}

#[cfg(test)]
#[path = "schemas_test.rs"]
mod tests;
