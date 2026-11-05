//! The constants of the `answers` concept: what a reason a language does not read a relation is called when a listing shows it.

use super::types::AbsenceCause;

/// The text of a reason a language does not read a relation.
#[must_use]
pub fn reason_name(reason: AbsenceCause) -> &'static str {
    match reason {
        AbsenceCause::NoSuchConstruct => "no-such-construct",
        AbsenceCause::NotCode => "not-code",
        AbsenceCause::NotAKeyedDocument => "not-a-keyed-document",
        AbsenceCause::NotADocument => "not-a-document",
        AbsenceCause::NotYetRead => "not-yet-read",
        AbsenceCause::NoManifests => "no-manifests",
    }
}

#[cfg(test)]
#[path = "constants_test.rs"]
mod tests;
