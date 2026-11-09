//! The `answers` concept's service: the text a reason a language does not read a relation is called by when a listing shows it.

use super::constants::{
    NO_MANIFESTS, NO_SUCH_CONSTRUCT, NOT_A_DOCUMENT, NOT_A_KEYED_DOCUMENT, NOT_CODE, NOT_YET_READ,
};
use super::types::AbsenceCause;
use crate::domain::terms::types::NameText;

/// The text of a reason a language does not read a relation.
#[must_use]
pub fn reason_name(reason: AbsenceCause) -> NameText<'static> {
    match reason {
        AbsenceCause::NoSuchConstruct => NO_SUCH_CONSTRUCT,
        AbsenceCause::NotCode => NOT_CODE,
        AbsenceCause::NotAKeyedDocument => NOT_A_KEYED_DOCUMENT,
        AbsenceCause::NotADocument => NOT_A_DOCUMENT,
        AbsenceCause::NotYetRead => NOT_YET_READ,
        AbsenceCause::NoManifests => NO_MANIFESTS,
    }
}
