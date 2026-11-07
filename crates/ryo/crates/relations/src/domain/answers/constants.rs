//! The constants of the `answers` concept: what a reason a language does not read a relation is called when a listing shows it.

use crate::domain::terms::types::NameText;

/// The text of the reason `NoSuchConstruct`.
pub const NO_SUCH_CONSTRUCT: NameText<'static> = NameText("no-such-construct");

/// The text of the reason `NotCode`.
pub const NOT_CODE: NameText<'static> = NameText("not-code");

/// The text of the reason `NotAKeyedDocument`.
pub const NOT_A_KEYED_DOCUMENT: NameText<'static> = NameText("not-a-keyed-document");

/// The text of the reason `NotADocument`.
pub const NOT_A_DOCUMENT: NameText<'static> = NameText("not-a-document");

/// The text of the reason `NotYetRead`.
pub const NOT_YET_READ: NameText<'static> = NameText("not-yet-read");

/// The text of the reason `NoManifests`.
pub const NO_MANIFESTS: NameText<'static> = NameText("no-manifests");
