//! Canonical source metadata framing and refusal vocabulary.
pub(super) const METADATA_HEADER: &[u8] = b"ryo-case-file-metadata-v1\0";
pub(super) const METADATA_PATH_LENGTH: &str = "metadata path length exceeds canonical encoding";
pub(super) const METADATA_ORDER: &str = "metadata paths must be unique and strictly ordered";
/// What joins the components of an artifact address wherever it is persisted or compared across machines.
pub(super) const SLASH: &str = "/";
