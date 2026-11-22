//! The text this part uses.
/// Text of the absolute context dependency in this part.
pub(super) const ABSOLUTE_CONTEXT_DEPENDENCY: &str = "absolute context dependency";
/// Text of the context logical and physical roots disagree in this part.
pub(super) const CONTEXT_LOGICAL_AND_PHYSICAL_ROOTS_DISAGREE: &str =
    "context logical and physical roots disagree";
/// Text of the duplicate missing context path in this part.
pub(super) const DUPLICATE_MISSING_CONTEXT_PATH: &str = "duplicate missing context path";
/// Text of the context path has conflicting roles in this part.
pub(super) const CONTEXT_PATH_HAS_CONFLICTING_ROLES: &str = "context path has conflicting roles";
/// Text of the sealed corpus requires unique actual tracked in this part.
pub(super) const SEALED_CORPUS_REQUIRES_UNIQUE_ACTUAL_TRACKED: &str =
    "sealed corpus requires unique actual tracked paths covering every requested source";
/// Text of the missing context parent in this part.
pub(super) const MISSING_CONTEXT_PARENT: &str = "missing context parent";
/// Text of the auxiliary file also requested for lint in this part.
pub(super) const AUXILIARY_FILE_ALSO_REQUESTED_FOR_LINT: &str =
    "auxiliary file also requested for lint";
/// Text of the missing source parent in this part.
pub(super) const MISSING_SOURCE_PARENT: &str = "missing source parent";
/// Text of the project dependency fingerprint disagrees in this part.
pub(super) const PROJECT_DEPENDENCY_FINGERPRINT_DISAGREES: &str =
    "project dependency fingerprint disagrees";
/// Refusal for a requested lint source that is a symbolic link.
pub(super) const SEALED_SOURCE_IS_A_LINK: &str = "requested lint source is a symbolic link";
/// Refusal for a link whose target is neither a sealed file nor a directory holding one, of the same snapshot.
pub(super) const CONTEXT_LINK_TARGET_UNSEALED: &str = "context link target is not a sealed file";
/// Refusal for a link to a directory that holds the link itself: a cycle.
pub(super) const CONTEXT_LINK_TARGET_HOLDS_THE_LINK: &str =
    "context link target is a directory that holds the link";
/// Refusal for link target text that is not canonical: empty, absolute, a repeated or trailing separator, `.`, or `..` after a name.
pub(super) const CONTEXT_LINK_TARGET_NOT_CANONICAL: &str =
    "context link target is not canonical relative text";
/// Refusal for link target text that is not UTF-8.
pub(super) const NON_UTF8_LINK_TARGET: &str = "context link target is not UTF-8";
/// Refusal for an observed dependency that names a sealed link.
pub(super) const CONTEXT_DEPENDENCY_IS_A_LINK: &str = "context dependency names a sealed link";
/// Text of the context receipt identity disagrees in this part.
pub(super) const CONTEXT_RECEIPT_IDENTITY_DISAGREES: &str = "context receipt identity disagrees";
