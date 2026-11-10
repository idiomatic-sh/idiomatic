//! The constants of the `answers` concept: what a reason a language does not read a relation is called when a listing shows it.

use crate::domain::terms::types::NameText;

/// The text of the reason `NoSuchConstruct`.
pub(crate) const NO_SUCH_CONSTRUCT: NameText<'static> = NameText("no-such-construct");

/// The text of the reason `NotCode`.
pub(crate) const NOT_CODE: NameText<'static> = NameText("not-code");

/// The text of the reason `NotAKeyedDocument`.
pub(crate) const NOT_A_KEYED_DOCUMENT: NameText<'static> = NameText("not-a-keyed-document");

/// The text of the reason `NotADocument`.
pub(crate) const NOT_A_DOCUMENT: NameText<'static> = NameText("not-a-document");

/// The text of the reason `NotYetRead`.
pub(crate) const NOT_YET_READ: NameText<'static> = NameText("not-yet-read");

/// The text of the reason `NoManifests`.
pub(crate) const NO_MANIFESTS: NameText<'static> = NameText("no-manifests");
