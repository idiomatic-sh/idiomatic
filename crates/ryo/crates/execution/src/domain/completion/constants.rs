//! Physical frontier refusal diagnostics.
pub(super) const FOREIGN_PARTITION: &str = "file frontier carries a foreign physical partition";
pub(super) const EMPTY_PARTIAL: &str = "partial frontier has no affected physical partition";
pub(super) const NO_AFFECTED_SCOPE: &str = "partial contributor has no affected physical scope";
